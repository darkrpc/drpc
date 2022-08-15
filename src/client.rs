use std::ops::DerefMut;
use std::time::Duration;
use crate::codec::Codec;
use crate::stub::ClientStub;
use dark_std::errors::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::balance::RpcClient;

use tokio::net::TcpStream;
use tokio::sync::Mutex;

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
    pub stream: Mutex<TcpStream>,
}

impl<C: Codec> Client<C> {
    pub async fn dial(addr: &str) -> std::io::Result<Self> {
        let address = addr.to_string();
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            addr: address,
            codec: C::default(),
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
        let resp: Resp = self.stub.call(func, arg, &self.codec, stream.deref_mut()).await?;
        return Ok(resp);
    }
}

impl<C: Codec> RpcClient for Client<C> {
    fn addr(&self) -> &str {
        self.addr.as_str()
    }
}