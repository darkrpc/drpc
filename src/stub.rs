use std::io::{BufReader, Read, Write};
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
use crate::frame::{Frame, ReqBuf, RspBuf, WireError};
use crate::server::Stub;
use dark_std::errors::Error;

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
    pub timeout: Duration,
    pub tag: AtomicU64,
}

impl ClientStub {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(60),
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
        req_buf.write_all(&arg_data)?;
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
        let data = req_buf.finish(id);
        stream.write_all(&data)?;
        let time = std::time::Instant::now();
        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame = Frame::decode_from(stream).map_err(|e| Error::from(e))?;
            // discard the rsp that is is not belong to us
            if rsp_frame.id == id {
                debug!("get response id = {}", id);
                let rsp_req = rsp_frame.decode_req();
                let rsp_data = rsp_frame.decode_rsp().map_err(|e| Error::from(e.to_string()))?;
                let resp: Resp = codec.decode(rsp_data)?;
                return Ok(resp);
            } else {
                if time.elapsed() > self.timeout {
                    return Err(err!("rpc call timeout!"));
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
    pub fn call(&self, stubs: &SyncHashMap<String, Box<dyn Stub>>, codec: &Codecs, mut stream: TcpStream) {
        // the read half of the stream
        let mut rs = BufReader::new(stream.try_clone().expect("failed to clone stream"));
        loop {
            let req = match Frame::decode_from(&mut rs) {
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
            let mut rsp = RspBuf::new();
            let req_data = req.decode_req();
            if let Ok(h) = codec.decode::<PackReq>(&req_data) {
                let stub = stubs.get(&h.m);
                if stub.is_none() {
                    let data = rsp.finish(req.id, Err(WireError::ClientDeserialize(format!("method {} not find!", h.m))));
                    debug!("send rsp: id={}", req.id);
                    // send the result back to client
                    stream.write(&data);
                    return;
                }
                let stub = stub.unwrap();
                let r = stub.accept(&h.body, codec);
                if let Err(e) = r {
                    let data = rsp.finish(req.id, Err(WireError::ClientDeserialize(format!("accept {} fail!", e))));
                    debug!("send rsp: id={}", req.id);
                    // send the result back to client
                    stream.write(&data);
                    continue;
                }
                let r = r.unwrap();
                rsp.write_all(&r);
            }
            // let ret = server.service(req.decode_req(), &mut rsp);
            let data = rsp.finish(req.id, Ok(()));
            debug!("send rsp ok: id={}", req.id);
            // send the result back to client
            stream.write(&data);
        }
    }
}