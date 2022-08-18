use std::future::Future;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use crate::tokio::io::AsyncWriteExt;
use std::pin::Pin;
use std::task::{Context, Poll};
use byteorder::{BigEndian, WriteBytesExt};
use log::debug;

// Frame layout
// id(u64) + ok(u8) + len(u64) + payload([u8; len])

// req frame layout
// id(u64) + ok(u8) + len(u64) + payload([u8; len])

// rsp frame layout(ok=0,payload is string,ok=1,payload is data)
// id(u64) + ok(u8) + len(u64) + payload/string ([u8; len])

/// raw frame wrapper, low level protocol
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Frame {
    /// frame id, req and rsp has the same id
    pub id: u64,
    /// is ok,false=0, true = 1
    pub ok: u8,
    /// payload data
    pub data: Vec<u8>,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            id: 0,
            ok: 0,
            data: vec![],
        }
    }

    /// decode a frame from the reader
    pub async fn decode_from<R: AsyncRead + Unpin>(r: &mut R) -> std::io::Result<Self> {
        let id = r.read_u64().await?;
        debug!("decode id = {:?}", id);

        let ok = r.read_u8().await?;
        debug!("decode ok = {:?}", ok);

        let len = r.read_u64().await?;
        debug!("decode len = {:?}", len);
        let mut datas = Vec::with_capacity(len as usize);
        unsafe { datas.set_len(len as usize) }; // it's safety,avoid one memset
        r.read_exact(&mut datas).await?;
        Ok(Frame { id, ok, data: datas })
    }

    /// decode a request/response from the frame, this would return the req raw buffer
    /// you need to deserialized from it into the real type
    pub fn get_payload(&self) -> &[u8] {
        &self.data
    }

    /// convert self into raw buf that can be send as a frame
    pub fn finish(self, id: u64) -> Vec<u8> {
        let len = self.data.len() as u64;
        let mut buf = Vec::with_capacity((17 + len) as usize);
        let _=WriteBytesExt::write_u64::<BigEndian>(&mut buf, id);
        let _=WriteBytesExt::write_u8(&mut buf, self.ok as u8);
        let _=WriteBytesExt::write_u64::<BigEndian>(&mut buf, len);
        buf.extend(self.data);
        buf
    }
}

impl AsyncWrite for Frame {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, std::io::Error>> {
        let mut p = Box::pin(AsyncWriteExt::write(&mut self.data, buf));
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

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}


#[cfg(test)]
mod test {
    use std::io::{Error, Write};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
    use crate::frame::{Frame};

    pub struct Mock {
        pub inner: Vec<u8>,
        pub pos: usize,
    }

    impl AsyncRead for Mock {
        fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
            buf.put_slice(&self.inner[self.pos..self.pos + buf.initialized().len()]);
            self.get_mut().pos += buf.filled().len();
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
        let mut req = Frame::new();
        let body = "hello".as_bytes();
        println!("body={:?}", body);
        req.write_all(body).await;
        let data = req.finish(100);
        let mut mock = Mock {
            inner: data,
            pos: 0,
        };
        println!("data={:?}", mock.inner);
        let f = Frame::decode_from(&mut mock).await.unwrap();
        println!("id={},ok={},data={:?}", f.id, f.ok, f.data);
    }
}