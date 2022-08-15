use std::future::Future;
use tokio::net::{TcpListener};
use crate::codec::{BinCodec, Codec};
use crate::stub::ServerStub;
use std::sync::Arc;
use dark_std::sync::SyncHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use dark_std::errors::Result;
use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct Server<C: Codec> {
    pub handles: SyncHashMap<String, Box<dyn Stub<C>>>,
    pub codec: C,
    pub stub: ServerStub,
}

impl<C: Codec> Server<C> {
    pub fn new() -> Self {
        Self {
            handles: SyncHashMap::new(),
            codec: C::default(),
            stub: ServerStub::new(),
        }
    }
}

impl Default for Server<BinCodec> {
    fn default() -> Self {
        Self {
            handles: SyncHashMap::new(),
            codec: BinCodec {},
            stub: ServerStub::new(),
        }
    }
}

impl<C: Codec> Server<C> {
    ///call server method
    #[inline]
    pub async fn call<S>(&self, stream: S) where S: AsyncRead + AsyncWrite + Unpin {
        self.stub.call(&self.handles, &self.codec, stream).await;
    }
}

pub trait Stub<C: Codec>: Sync + Send {
    fn accept(&self, arg: &[u8], codec: &C) -> BoxFuture<Result<Vec<u8>>>;
}

pub trait Handler<C: 'static + Codec>: Stub<C> + Sync + Send {
    type Req: DeserializeOwned + Send;
    type Resp: Serialize;
    fn accept(&self, arg: &[u8], codec: &C) -> BoxFuture<Result<Vec<u8>>> {
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
            let f = f?;
            let data = f.await?;
            Ok(codec.encode(data)?)
        })
    }
    fn handle(&self, req: Self::Req) -> BoxFuture<Result<Self::Resp>>;
}

impl<C: Codec + 'static, H: Handler<C>> Stub<C> for H {
    fn accept(&self, arg: &[u8], codec: &C) -> BoxFuture<Result<Vec<u8>>> {
        <H as Handler::<C>>::accept(self, arg, codec)
    }
}

pub struct HandleFn<Req: DeserializeOwned, Resp: Serialize> {
    pub f: Box<dyn Fn(Req) -> BoxFuture<'static, Result<Resp>>>,
}
//it's safety
unsafe impl<Req: DeserializeOwned, Resp: Serialize> Sync for HandleFn<Req, Resp> {}
//it's safety
unsafe impl<Req: DeserializeOwned, Resp: Serialize> Send for HandleFn<Req, Resp> {}

impl<C: Codec + 'static, Req: DeserializeOwned + Send, Resp: Serialize> Handler<C> for HandleFn<Req, Resp> {
    type Req = Req;
    type Resp = Resp;

    fn handle(&self, req: Self::Req) -> BoxFuture<dark_std::errors::Result<Self::Resp>> {
        (self.f)(req)
    }
}

impl<Req: DeserializeOwned, Resp: Serialize> HandleFn<Req, Resp> {
    pub fn new<F: 'static>(f: F) -> Self where F: Fn(Req) -> BoxFuture<'static, Result<Resp>> {
        Self {
            f: Box::new(f),
        }
    }
}


impl<C: Codec + 'static> Server<C> {
    ///register a handle to server
    pub async fn register<H: 'static>(&mut self, name: &str, handle: H) where H: Stub<C> {
        self.handles.insert(name.to_owned(), Box::new(handle)).await;
    }

    /// register a register_box_future into server
    pub fn register_box_future<Req: DeserializeOwned + Send + 'static, Resp: Serialize + 'static, F: 'static>(&mut self, name: &str, f: F)
        where F: Fn(Req) -> BoxFuture<'static, Result<Resp>> {
        self.handles.insert_mut(name.to_owned(), Box::new(HandleFn::new(f)));
    }

    /// register a func into server
    /// for example:
    /// ```
    /// use drpc::server::{Server};
    /// use dark_std::errors::Result;
    /// let mut s = Server::default();
    /// async fn handle(req: i32) -> dark_std::errors::Result<i32> { return Ok(req + 1); }
    ///
    ///     s.register_fn("handle", handle);
    ///     //way 2
    ///     s.register_fn("handle_fn2", |arg:i32| async move {
    ///         Ok(1)
    ///     });
    /// ```
    pub fn register_fn<Req: DeserializeOwned + Send + 'static, Resp: Serialize + 'static, Out: 'static, F: 'static>(&mut self, name: &str, f: F)
        where
            Out: Future<Output=Result<Resp>> + Send,
            F: Fn(Req) -> Out {
        self.handles.insert_mut(name.to_owned(), Box::new(HandleFn::new(move |req: Req| -> BoxFuture<'static, Result<Resp>>{
            Box::pin((f)(req))
        })));
    }

    pub async fn serve<A>(self, addr: A) where A: tokio::net::ToSocketAddrs {
        let listener = TcpListener::bind(addr).await.unwrap();
        println!(
            "Starting tcp server on {:?}",
            listener.local_addr().unwrap(),
        );
        let server = Arc::new(self);
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let server = server.clone();
                tokio::spawn(async move {
                    server.call(stream).await;
                });
            }
        }
    }
}