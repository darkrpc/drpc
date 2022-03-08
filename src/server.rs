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
use std::process::Output;
use std::sync::Arc;
use log::error;
use dark_std::sync::SyncHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use dark_std::errors::Result;
use futures::future::BoxFuture;

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
    fn accept(&self, arg: &[u8], codec: &Codecs) -> BoxFuture<Result<Vec<u8>>>;
}

pub trait Handler: Stub + Sync + Send {
    type Req: DeserializeOwned + Send;
    type Resp: Serialize;
    fn accept(&self, arg: &[u8], codec: &Codecs) -> BoxFuture<Result<Vec<u8>>> {
        let req = codec.decode::<Self::Req>(arg);
        let f = {
            if req.is_err() {
                Err(req.err().unwrap())
            } else {
                Ok(self.handle(req.unwrap()))
            }
        };
        let codec = codec.clone();
        Box::pin(async move {
            let mut f = f?;
            let data = f.await?;
            Ok(codec.encode(data)?)
        })
    }
    fn handle(&self, req: Self::Req) -> BoxFuture<Result<Self::Resp>>;
}

impl<H: Handler> Stub for H {
    fn accept(&self, arg: &[u8], codec: &Codecs) -> BoxFuture<Result<Vec<u8>>> {
        <H as Handler>::accept(self, arg, codec)
    }
}

pub struct HandleFn<Req: DeserializeOwned, Resp: Serialize> {
    pub f: Box<dyn Fn(Req) -> BoxFuture<'static,Result<Resp>>>,
}

unsafe impl<Req: DeserializeOwned, Resp: Serialize> Sync for HandleFn<Req, Resp> {}

unsafe impl<Req: DeserializeOwned, Resp: Serialize> Send for HandleFn<Req, Resp> {}

impl<Req: DeserializeOwned + Send, Resp: Serialize> Handler for HandleFn<Req, Resp> {
    type Req = Req;
    type Resp = Resp;

    fn handle(&self, req: Self::Req) -> BoxFuture<dark_std::errors::Result<Self::Resp>> {
        (self.f)(req)
    }
}

impl<Req: DeserializeOwned, Resp: Serialize> HandleFn<Req, Resp> {
    pub fn new<F: 'static>(f: F) -> Self where F: Fn(Req) -> BoxFuture<'static,Result<Resp>> {
        Self {
            f: Box::new(f),
        }
    }
}


impl Server {
    ///register a handle to server
    pub async fn register<H: 'static>(&mut self, name: &str, handle: H) where H: Stub {
        self.handles.insert(name.to_owned(), Box::new(handle)).await;
    }

    /// register a func into server
    /// for example:
    /// ```
    /// use drpc::server::{Server};
    /// use dark_std::errors::Result;
    /// let mut s = Server::default();
    /// async fn handle(req: i32) -> dark_std::errors::Result<i32> { return Ok(req + 1); }
    ///     //s.codec = Codecs::JsonCodec(JsonCodec{});
    ///     s.register_fn("handle", handle);
    ///     //way 2
    ///     s.register_fn("handle_fn2", |arg:i32| async move {
    ///         Ok(1)
    ///     });
    /// ```
    pub fn register_box_future<Req: DeserializeOwned + Send + 'static, Resp: Serialize + 'static, F: 'static>(&mut self, name: &str, f: F)
        where F: Fn(Req) -> BoxFuture<'static,Result<Resp>> {
        self.handles.insert_mut(name.to_owned(), Box::new(HandleFn::new(f)));
    }

    ///register async fn
    pub fn register_fn<Req: DeserializeOwned + Send + 'static, Resp: Serialize + 'static,Out:'static, F: 'static>(&mut self, name: &str, f: F)
        where
            Out:Future<Output=Result<Resp>>+Send,
            F: Fn(Req) -> Out {
        let f1= move |req:Req| -> BoxFuture<'static,Result<Resp>>{
            Box::pin((f)(req))
        };
        self.handles.insert_mut(name.to_owned(), Box::new(HandleFn::new(f1)));
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