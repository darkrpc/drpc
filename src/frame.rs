use std::future::Future;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader};
use crate::tokio::io::AsyncWriteExt;
use std::io::{self, Cursor, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use dark_std::errors::Error;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error, info};

// Frame layout
// id(u64) + len(u64) + payload([u8; len])

// req frame layout
// id(u64) + len(u64) + req_data([u8; len])

// rsp frame layout
// id(u64) + len(u64) + ty(u8) + len1(u64) + rsp_data([u8; len1])

// max frame len
const FRAME_MAX_LEN: u64 = 1024 * 1024;

/// raw frame wrapper, low level protocol
/// TODO: add check sum check
#[derive(Debug)]
pub struct Frame {
    /// frame id, req and rsp has the same id
    pub id: u64,
    /// payload data
    data: Vec<u8>,
}

impl Frame {
    /// decode a frame from the reader
    pub async fn decode_from<R: AsyncRead + Unpin>(r: &mut R) -> io::Result<Self> {
        let id = r.read_u64().await?;
        debug!("decode id = {:?}", id);

        let len = r.read_u64().await? + 16;
        debug!("decode len = {:?}", len);

        if len > FRAME_MAX_LEN {
            let s = format!("decode too big frame length. len={}", len);
            error!("{}", s);
            return Err(io::Error::new(ErrorKind::InvalidInput, s));
        }

        let mut data = Vec::with_capacity(len as usize);
        unsafe { data.set_len(len as usize) }; // avoid one memset
        r.read_exact(&mut data[16..]).await?;

        // blow can be skipped, we don't need them in the buffer
        let mut cursor = Cursor::new(data);
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, id).unwrap();
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, len - 16).unwrap();
        let data = cursor.into_inner();

        Ok(Frame { id, data })
    }

    /// convert self into raw buf that can be re-send as a frame
    // pub fn finish(self, id: u64) -> Vec<u8> {
    //     let mut cursor = Cursor::new(self.data);

    //     // write from start
    //     cursor.write_u64::<BigEndian>(id).unwrap();
    //     info!("re-encode id = {:?}", id);

    //     cursor.into_inner()
    // }

    /// decode a request from the frame, this would return the req raw bufer
    /// you need to deserialized from it into the real type
    pub fn decode_req(&self) -> &[u8] {
        // skip the frame head
        &self.data[16..]
    }

    /// decode a response from the frame, this would return the rsp raw bufer
    /// you need to deserialized from it into the real type
    pub fn decode_rsp(&self) -> Result<&[u8], WireError> {
        let mut r = Cursor::new(&self.data);
        // skip the frame head
        r.set_position(16);

        let ty = ReadBytesExt::read_u8(&mut r)?;
        // we don't need to check len here, frame is checked already
        let len = ReadBytesExt::read_u64::<BigEndian>(&mut r)? as usize;

        let buf = r.into_inner();
        let data = &buf[25..len + 25];

        // info!("decode response, ty={}, len={}", ty, len);
        match ty {
            0 => Ok(data),
            1 => Err(WireError::ServerDeserialize(unsafe {
                String::from_utf8_unchecked(data.into())
            })),
            2 => Err(WireError::ServerSerialize(unsafe {
                String::from_utf8_unchecked(data.into())
            })),
            3 => Err(WireError::Status(unsafe { String::from_utf8_unchecked(data.into()) })),
            _ => {
                let s = format!("invalid response type. ty={}", ty);
                error!("{}", s);
                Err(WireError::ClientDeserialize(s))
            }
        }
    }
}

/// req frame buffer that can be serialized into
pub struct ReqBuf(Cursor<Vec<u8>>);

impl Default for ReqBuf {
    fn default() -> Self {
        ReqBuf::new()
    }
}

impl ReqBuf {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(128);
        buf.resize(16, 0);
        let mut cursor = Cursor::new(buf);
        // leave enough space to write id and len
        cursor.set_position(16);
        ReqBuf(cursor)
    }

    /// convert self into raw buf that can be send as a frame
    pub fn finish(self, id: u64) -> Vec<u8> {
        let mut cursor = self.0;
        let len = cursor.get_ref().len() as u64;
        assert!(len <= FRAME_MAX_LEN);

        // write from start
        cursor.set_position(0);
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, id).unwrap();
        debug!("encode id = {:?}", id);

        // adjust the data length
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, len - 16).unwrap();
        debug!("encode len = {:?}", len);

        cursor.into_inner()
    }
}

impl AsyncWrite for ReqBuf {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        let mut p = Box::pin(AsyncWriteExt::write(&mut self.0, buf));
        loop {
            match Pin::new(&mut p).poll(cx) {
                Poll::Ready(v) => {
                    match v {
                        Ok(v) => {
                            return Poll::Ready(Ok(v));
                        }
                        Err(e) => {
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                Poll::Pending => {}
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

/// rsp frame buffer that can be serialized into
pub struct RspBuf(Cursor<Vec<u8>>);

impl Default for RspBuf {
    fn default() -> Self {
        RspBuf::new()
    }
}

pub const SERVER_POLL_ENCODE: u8 = 200;

impl RspBuf {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(64);
        // id + len + ty + len + data
        buf.resize(25, 0);
        let mut cursor = Cursor::new(buf);
        // leave enough space to write id and len
        cursor.set_position(25);
        RspBuf(cursor)
    }

    /// convert self into raw buf that can be send as a frame
    pub async fn finish(self, id: u64, ret: Result<(), WireError>) -> Vec<u8> {
        let mut cursor = self.0;
        let dummy = Vec::new();

        let (ty, len, data) = match ret {
            Ok(_) => (0, cursor.get_ref().len() - 25, dummy.as_slice()),
            Err(ref e) => match *e {
                WireError::ServerDeserialize(ref s) => (1, s.len(), s.as_bytes()),
                WireError::ServerSerialize(ref s) => (2, s.len(), s.as_bytes()),
                WireError::Status(ref s) => (3, s.len(), s.as_bytes()),
                WireError::Polling => (SERVER_POLL_ENCODE, 0, dummy.as_slice()),
                WireError::ClientDeserialize(ref s) => (2, s.len(), s.as_bytes()),
            },
        };

        let len = len as u64;
        assert!(len < FRAME_MAX_LEN);

        // write from start
        cursor.set_position(0);
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, id).unwrap();
        debug!("encode id = {:?}", id);

        // adjust the data length
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, len + 9).unwrap();
        debug!("encode len = {:?}", len);

        // write the type
        WriteBytesExt::write_u8(&mut cursor, ty).unwrap();
        // write the len
        WriteBytesExt::write_u64::<BigEndian>(&mut cursor, len).unwrap();
        // write the data into the writer
        match ty {
            0 => {} // the normal ret already writed
            SERVER_POLL_ENCODE => {
                // the server need to poll the client, will be filtered out by multiplex_client
            }
            1 | 2 | 3 => {
                cursor.get_mut().resize(len as usize + 25, 0);
                cursor.write_all(data).await.unwrap();
            }
            _ => unreachable!("unkown rsp type"),
        }

        cursor.into_inner()
    }
}

impl AsyncWrite for RspBuf {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        let mut p = Box::pin(AsyncWriteExt::write(&mut self.0, buf));
        loop {
            match Pin::new(&mut p).poll(cx) {
                Poll::Ready(v) => {
                    match v {
                        Ok(v) => {
                            return Poll::Ready(Ok(v));
                        }
                        Err(e) => {
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                Poll::Pending => {}
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}


/// A serializable, server-supplied error.
#[doc(hidden)]
#[derive(Debug)]
pub enum WireError {
    ClientDeserialize(String),
    /// Error in deserializing a client request.
    ServerDeserialize(String),
    /// Error in serializing server response.
    ServerSerialize(String),
    /// Server Status
    Status(String),
    /// Server polling
    /// this is a special error code that used for server polling request from client
    /// client will first check this code in the very beginning before return to client rpc call
    Polling,
}

impl From<std::io::Error> for WireError {
    fn from(e: io::Error) -> Self {
        Self::ServerDeserialize(e.to_string())
    }
}

impl ToString for WireError {
    fn to_string(&self) -> String {
        match self {
            WireError::ClientDeserialize(e) => { e.to_owned() }
            WireError::ServerDeserialize(e) => { e.to_owned() }
            WireError::ServerSerialize(e) => { e.to_owned() }
            WireError::Status(e) => { e.to_owned() }
            WireError::Polling => { "Polling".to_string() }
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::{Error, Write};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
    use crate::frame::{Frame, ReqBuf};

    pub struct Mock {
        inner: Vec<u8>,
    }

    impl AsyncRead for Mock {
        fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
            buf.put_slice(&self.inner[0..buf.initialized().len()]);
            return Poll::Ready(Ok(()));
        }
    }

    impl AsyncWrite for Mock {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
            self.get_mut().inner.extend(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_frame() {
        let mut req = ReqBuf::new();
        let body = "hello".as_bytes();
        req.write_all(body);
        let data = req.finish(1);
        let mut mock = Mock {
            inner: data
        };
        let f = Frame::decode_from(&mut mock).await.unwrap();
        println!("id={},data={:?}", f.id, f.data);
    }
}