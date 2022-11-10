use dark_std::errors::Error;
use dark_std::errors::Result;
use dark_std::sync::map_hash::SyncHashMap;
use log::{debug, error};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::codec::Codec;
use crate::frame::Frame;
use crate::server::Stub;

/// Pack the client request parameters into a network message,
/// which is then sent to the server remotely over the network.
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

    pub async fn call_frame<C: Codec, Arg: Serialize, Resp: DeserializeOwned, F, Transport>(
        &self,
        method: &str,
        arg: Arg,
        codec: &C,
        transport: Transport,
    ) -> Result<Resp>
    where
        F: Future<Output = Frame>,
        Transport: FnOnce(Frame) -> F,
    {
        let mut arg_data = method.to_string().into_bytes();
        arg_data.push('\n' as u8);
        arg_data.extend(codec.encode(arg)?);
        let mut req_buf = Frame::new();
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
        let rsp_frame = transport(req_buf).await;
        // discard the rsp that is is not belong to us
        debug!("get response id = {}", id);
        if rsp_frame.ok == 0 {
            let rsp_data = rsp_frame.get_payload();
            //it's safety.rsp_data when ok = 0 must be string(utf8) data
            let resp: String = unsafe { String::from_utf8_unchecked(rsp_data.to_vec()) };
            return Err(Error { inner: resp });
        } else {
            let rsp_data = rsp_frame.get_payload();
            let resp: Resp = codec.decode(rsp_data)?;
            return Ok(resp);
        }
    }

    pub async fn call<C: Codec, Arg: Serialize, Resp: DeserializeOwned, S>(
        &self,
        method: &str,
        arg: Arg,
        codec: &C,
        mut stream: S,
    ) -> Result<Resp>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.call_frame(method, arg, codec, move |req_buf: Frame| async move {
            let id = req_buf.id;
            let data = req_buf.finish(id);
            if let Err(e) = stream.write_all(&data).await {
                return Frame {
                    id,
                    ok: 0,
                    data: e.to_string().into_bytes(),
                };
            }
            let mut time_start = None;
            if self.timeout.is_some() {
                time_start = Some(std::time::Instant::now());
            }
            loop {
                // deserialize the rsp
                let rsp_frame = Frame::decode_from(&mut stream)
                    .await
                    .map_err(|e| Error::from(e));
                if rsp_frame.is_err() {
                    return Frame {
                        id,
                        ok: 0,
                        data: rsp_frame.err().unwrap().to_string().into_bytes(),
                    };
                }
                let rsp_frame = rsp_frame.unwrap();
                // discard the rsp that is is not belong to us
                if rsp_frame.id == id {
                    debug!("get response id = {}", id);
                    return rsp_frame;
                }
                if let Some(timeout) = self.timeout {
                    if let Some(time_start) = &time_start {
                        if time_start.elapsed() > timeout {
                            return Frame {
                                id,
                                ok: 0,
                                data: "rpc call timeout!".to_string().into_bytes(),
                            };
                        }
                    }
                }
            }
        })
        .await
    }
}

/// Receives the message sent by the client, unpacks the message, and invokes the local method.
pub struct ServerStub {}

impl ServerStub {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn call_frame<C: Codec>(
        &self,
        stubs: &SyncHashMap<String, Box<dyn Stub<C>>>,
        codec: &C,
        req: Frame,
    ) -> Frame {
        let mut rsp = Frame::new();
        let payload = req.get_payload();
        let method = {
            let mut find_end = false;
            let mut method = String::with_capacity(20);
            for x in payload {
                if x.eq(&('\n' as u8)) {
                    find_end = true;
                    break;
                }
                method.push(*x as char);
            }
            if !find_end {
                let _ = rsp
                    .write_all("not find '\n' end of method!".as_bytes())
                    .await;
                rsp.ok = 0;
                return rsp;
            }
            method
        };
        let stub = stubs.get(&method);
        if stub.is_none() {
            let _ = rsp
                .write_all(format!("method='{}' not find!", method).as_bytes())
                .await;
            rsp.ok = 0;
            return rsp;
        }
        let stub = stub.unwrap();
        let body = &payload[(method.len() + 1)..];
        let r = stub.accept(body, codec).await;
        if let Err(e) = r {
            let _ = rsp.write_all(e.to_string().as_bytes()).await;
            rsp.ok = 0;
            return rsp;
        }
        let r = r.unwrap();
        let _ = rsp.write_all(&r).await;
        rsp.ok = 1;
        rsp
    }

    pub async fn call<S, C: Codec>(
        &self,
        stubs: &SyncHashMap<String, Box<dyn Stub<C>>>,
        codec: &C,
        mut stream: S,
    ) where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        // the read half of the stream
        loop {
            let req = match Frame::decode_from(&mut stream).await {
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
            let id = req.id;
            debug!("req: id={:?}", id);
            let rsp = self.call_frame(stubs, codec, req).await;
            // let ret = server.service(req.decode_req(), &mut rsp);
            let data = rsp.finish(id);
            debug!("rsp: id={}", id);
            // send the result back to client
            let _ = stream.write(&data).await;
        }
    }
}
