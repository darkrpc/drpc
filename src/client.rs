use dark_std::errors::{Error, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::DerefMut;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::balance::RpcClient;
use crate::codec::Codec;
use crate::stub::ClientStub;

//TODO parse addr: tcp://addr,http://addr
//TODO use transport
/// a rpc client impl
///
/// use example:
/// ```rust
/// use drpc::client::Client;
/// use drpc::codec::BinCodec;
///
/// async fn test(){
///         let c = Client::<BinCodec>::dial("127.0.0.1:10000").await.unwrap();
///         println!("dial success");
///         let resp: i32 = c.call("handle", 1).await.unwrap();
///         println!("resp=>>>>>>>>>>>>>> :{}", resp);
/// }
///
/// ```
#[derive(Debug)]
pub struct Client<C: Codec> {
    pub addr: String,
    pub codec: C,
    pub stub: ClientStub,
    pub stream: Option<Mutex<TcpStream>>,
}

impl<C: Codec> Client<C> {
    pub async fn dial(addr: &str) -> std::io::Result<Self> {
        let address = addr.to_string();
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            addr: address,
            codec: C::default(),
            stub: ClientStub::new(),
            stream: Some(Mutex::new(stream)),
        })
    }

    /// Set the client's timeout.
    pub fn set_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.stub.timeout = timeout;
        self
    }

    /// Get the client's timeout.
    pub fn get_timeout(&self) -> &Option<Duration> {
        &self.stub.timeout
    }

    pub async fn call<Arg, Resp>(&self, func: &str, arg: Arg) -> Result<Resp>
    where
        Arg: Serialize,
        Resp: DeserializeOwned,
    {
        return if let Some(v) = self.stream.as_ref() {
            let mut stream = v.lock().await;
            let resp: Resp = self
                .stub
                .call(func, arg, &self.codec, stream.deref_mut())
                .await?;
            Ok(resp)
        } else {
            Err(Error::from("stream is shutdown!"))
        };
    }

    /// Shutdown the client.
    pub async fn shutdown(&mut self) {
        if let Some(v) = self.stream.take() {
            let mut stream = v.into_inner();
            let _ = stream.shutdown().await;
        }
    }
}

impl<C: Codec> RpcClient for Client<C> {
    fn addr(&self) -> &str {
        self.addr.as_str()
    }
}

impl<C: Codec> Drop for Client<C> {
    fn drop(&mut self) {
        if let Some(v) = self.stream.take() {
            let mut stream = v.into_inner();
            tokio::spawn(async move {
                let _ = stream.shutdown().await;
            });
        }
    }
}
