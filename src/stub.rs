use std::cell::{RefCell, RefMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use std::ops::Index;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use dark_std::err;
use log::{error, debug};
use dark_std::errors::Result;
use dark_std::sync::map_hash::SyncHashMap;
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use crate::codec::{Codec, Codecs};
use crate::frame::{Frame, ReqBuf};
use crate::server::Stub;
use dark_std::errors::Error;

use tokio::net::TcpStream;

#[derive(Serialize, Deserialize, Debug)]
pub struct PackReq {
    //method
    pub m: String,
    //method
    pub body: Vec<u8>,
}

/// and the client request parameters are packaged into a network message,
/// which is then sent to the server remotely over the network
#[derive(Debug)]
pub struct ClientStub {
    pub timeout: Option<Duration>,
    pub tag: AtomicU64,
}

impl ClientStub {
    pub fn new() -> Self {
        Self {
            timeout: None,
            tag: AtomicU64::new(0),
        }
    }

    pub async fn call<Arg: Serialize, Resp: DeserializeOwned>(&self, method: &str, arg: Arg, codec: &Codecs, stream: &mut TcpStream) -> Result<Resp> {
        let arg = PackReq {
            m: method.to_string(),
            body: codec.encode(arg)?,
        };
        let arg_data = codec.encode(arg)?;
        let mut req_buf = ReqBuf::new();
        req_buf.write_all(&arg_data).await?;
        let id = {
            let mut id = self.tag.load(Ordering::SeqCst);
            if id == u64::MAX {
                id = 0;
            } else {
                id += 1;
            }
            self.tag.store(id, Ordering::SeqCst);
            id
        };
        debug!("request id = {}", id);
        let data = req_buf.finish(id, true);
        stream.write_all(&data).await?;
        let time = std::time::Instant::now();
        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame = Frame::decode_from(stream).await.map_err(|e| Error::from(e))?;
            // discard the rsp that is is not belong to us
            if rsp_frame.id == id {
                debug!("get response id = {}", id);
                if rsp_frame.ok == 0 {
                    let rsp_data = rsp_frame.get_payload();
                    let resp: String = unsafe { String::from_utf8_unchecked(rsp_data.to_vec()) };
                    return Err(Error { inner: resp });
                } else {
                    let rsp_data = rsp_frame.get_payload();
                    let resp: Resp = codec.decode(rsp_data)?;
                    return Ok(resp);
                }
            } else {
                if let Some(timeout) = self.timeout{
                    if time.elapsed() > timeout {
                        return Err(err!("rpc call timeout!"));
                    }
                }
            }
        }
    }
}

/// Receives the message sent by the client, unpacks the me ssage, and invokes the local method.
pub struct ServerStub {}

impl ServerStub {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn call<S>(&self, stubs: &SyncHashMap<String, Box<dyn Stub>>, codec: &Codecs, mut stream: S) where S: AsyncRead + AsyncWrite + Unpin{
        // the read half of the stream
        let rd: S = unsafe { std::mem::transmute_copy(&stream) };
        let mut rs = SafeDrop::new(rd);
        loop {
            let req = match Frame::decode_from(rs.get_mut()).await {
                Ok(r) => r,
                Err(ref e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        debug!("tcp server decode req: connection closed");
                    } else {
                        error!("tcp server decode req: err = {:?}", e);
                    }
                    break;
                }
            };
            debug!("get request: id={:?}", req.id);
            let mut rsp = ReqBuf::new();
            let req_data = req.get_payload();
            if let Ok(h) = codec.decode::<PackReq>(&req_data) {
                let stub = stubs.get(&h.m);
                if stub.is_none() {
                    rsp.write_all(format!("method {} not find!", h.m).as_bytes()).await;
                    let data = rsp.finish(req.id, false);
                    debug!("send rsp: id={}", req.id);
                    // send the result back to client
                    stream.write(&data).await;
                    return;
                }
                let stub = stub.unwrap();
                let r = stub.accept(&h.body, codec).await;
                if let Err(e) = r {
                    rsp.write_all(e.to_string().as_bytes()).await;
                    let data = rsp.finish(req.id, false);
                    debug!("send rsp: id={}", req.id);
                    // send the result back to client
                    stream.write(&data).await;
                    continue;
                }
                let r = r.unwrap();
                rsp.write_all(&r).await;
            }
            // let ret = server.service(req.decode_req(), &mut rsp);
            let data = rsp.finish(req.id, true);
            debug!("send rsp ok: id={}", req.id);
            // send the result back to client
            stream.write(&data).await;
        }
    }
}

pub struct SafeDrop<S: AsyncRead + AsyncWrite + Unpin> {
    inner: Option<BufReader<S>>,
}

impl <S: AsyncRead + AsyncWrite + Unpin>Drop for SafeDrop<S> {
    fn drop(&mut self) {
        let v = self.inner.take().unwrap().into_inner();
        std::mem::forget(v);
    }
}

impl <S: AsyncRead + AsyncWrite + Unpin>SafeDrop<S> {
    pub fn new(tcp: S) -> SafeDrop<S> {
        Self {
            inner: Some(BufReader::new(tcp))
        }
    }
    fn get_mut(&mut self) -> &mut S {
        self.inner.as_mut().unwrap().get_mut()
    }
}