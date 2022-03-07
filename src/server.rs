use std::any::Any;
use std::future::Future;
use dark_std::err;
use tokio::net::{TcpListener, TcpStream};
use crate::codec::{BinCodec, Codec, Codecs};
use crate::stub::ServerStub;
use std::io::Read;
use std::io::Write;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use log::error;
use dark_std::sync::SyncHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use dark_std::errors::Result;

pub struct Server {
    pub handles: SyncHashMap<String, Box<dyn Stub>>,
    pub codec: Codecs,
    pub stub: ServerStub,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            handles: SyncHashMap::new(),
            codec: Codecs::BinCodec(BinCodec {}),
            stub: ServerStub::new(),
        }
    }
}

impl Server {
    #[inline]
    pub async fn call(&self, stream: TcpStream) {
        self.stub.call(&self.handles, &self.codec, stream).await;
    }
}

pub trait Stub: Sync + Send {
    fn accept(&self, arg: &[u8], codec: &Codecs) -> Pin<Box<dyn Future<Output=Result<Vec<u8>>> + Send>>;
}

pub trait Handler: Stub + Sync + Send + Clone + 'static {
    type Req: DeserializeOwned + Send;
    type Resp: Serialize;
    type Future: Future<Output=Result<Self::Resp>> + Send;
    fn accept(&self, arg: &[u8], codec: &Codecs) -> Pin<Box<dyn Future<Output=Result<Vec<u8>>> + Send>> {
        let codec = codec.clone();
        let arg = arg.to_owned();
        let s = self.clone();
        Box::pin(async move {
            let arg = arg;
            let req: Self::Req = codec.decode(&arg)?;
            let data = s.handle(req).await?;
            Ok(codec.encode(data)?)
        })
    }
    fn handle(&self, req: Self::Req) -> Self::Future;
}

impl<H: Handler> Stub for H {
    fn accept(&self, arg: &[u8], codec: &Codecs) -> Pin<Box<dyn Future<Output=Result<Vec<u8>>> + Send>> {
        <H as Handler>::accept(self, arg, codec)
    }
}


pub struct HandleFn<Req: DeserializeOwned, Resp: Serialize> {
    pub f: Arc<Pin<Box<dyn Fn(Req) -> dyn Future<Output=Result<Resp>>>>>,
}

unsafe impl<Req: DeserializeOwned, Resp: Serialize> Sync for HandleFn<Req, Resp> {}

unsafe impl<Req: DeserializeOwned, Resp: Serialize> Send for HandleFn<Req, Resp> {}

impl<Req: DeserializeOwned, Resp: Serialize> Clone for HandleFn<Req, Resp> {
    fn clone(&self) -> Self {
        HandleFn{
            f: self.f.clone()
        }
    }
}

impl<Req: DeserializeOwned+Send+'static, Resp: Serialize+'static> Handler for HandleFn<Req, Resp> {
    type Req = Req;
    type Resp = Resp;

    type Future =  Pin<Box<(dyn Future<Output = Result<Resp>> + Send)>>;

    fn handle(&self, req: Self::Req) -> Pin<Box<(dyn Future<Output = Result<Resp>> + Send)>> {
        // (self.f)(req)
        todo!()
    }
}

impl<Req: DeserializeOwned, Resp: Serialize> HandleFn<Req, Resp> {
    pub fn new<F: 'static>(f: F) -> Self where F: Fn(Req) -> dyn Future<Output=Result<Resp>>{
        Self {
            f: Arc::new(Box::pin(f)),
        }
    }
}


impl Server {
    ///register a handle to server
    /// ```
    /// use drpc::server::{Handler};
    /// use dark_std::errors::Result;
    ///
    /// pub struct H{}
    ///
    /// impl Handler for H{
    ///     type Req = i32;
    ///     type Resp = i32;
    ///
    ///     fn handle(&self, req: Self::Req) -> Result<Self::Resp> {
    ///         return Ok(req);
    ///     }
    /// }
    ///
    /// ```
    pub async fn register<H: 'static>(&mut self, name: &str, handle: H) where H: Stub {
        self.handles.insert(name.to_owned(), Box::new(handle)).await;
    }

    /// register a func into server
    /// for example:
    /// ```
    /// use drpc::server::{Server};
    /// use dark_std::errors::Result;
    /// let mut s = Server::default();
    /// fn handle(req: i32) -> dark_std::errors::Result<i32> {
    ///     return Ok(req + 1);
    /// }
    ///     //s.codec = Codecs::JsonCodec(JsonCodec{});
    ///     s.register_fn("handle", handle);
    ///     s.register_fn("handle_fn2", |arg:i32| -> Result<i32>{
    ///         Ok(1)
    ///     });
    /// ```
    pub async fn register_fn<Req: DeserializeOwned +Send+ 'static, Resp: Serialize + 'static, F: 'static>(&mut self, name: &str, f: F)
        where F: Fn(Req) -> dyn Future<Output=Result<Resp>> {
        self.handles.insert(name.to_owned(), Box::new(HandleFn::new(f))).await;
    }

    pub async fn serve<A>(self, addr: A) where A: tokio::net::ToSocketAddrs {
        let listener = TcpListener::bind(addr).await.unwrap();
        println!(
            "Starting tcp echo server on {:?}",
            listener.local_addr().unwrap(),
        );
        let server = Arc::new(self);
        for (stream, addr) in listener.accept().await {
            let server = server.clone();
            tokio::spawn(async move {
                server.call(stream).await;
            });
        }
    }
}