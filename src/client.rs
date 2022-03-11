use std::cell::RefCell;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use crate::codec::{BinCodec, Codec, Codecs};
use crate::stub::ClientStub;
use dark_std::errors::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::balance::RpcClient;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Client {
    pub addr: String,
    pub codec: Codecs,
    pub stub: ClientStub,
    pub stream: Mutex<TcpStream>,
}

impl Client {
    pub async fn dial(addr: &str) -> std::io::Result<Self> {
        let address = addr.to_string();
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            addr: address,
            codec: Codecs::BinCodec(BinCodec {}),
            stub: ClientStub::new(),
            stream: Mutex::new(stream),
        })
    }

    /// set timeout
    pub fn set_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.stub.timeout = timeout;
        self
    }

    /// get timeout
    pub fn get_timeout(&self) -> &Option<Duration> {
        &self.stub.timeout
    }

    pub async fn call<Arg, Resp>(&self, func: &str, arg: Arg) -> Result<Resp> where Arg: Serialize, Resp: DeserializeOwned {
        let mut stream = self.stream.lock().await;
        let resp: Resp = self.stub.call(func, arg, &self.codec, &mut stream).await?;
        return Ok(resp);
    }
}

impl RpcClient for Client {
    fn addr(&self) -> &str {
        self.addr.as_str()
    }
}