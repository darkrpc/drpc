use std::any::Any;
use dark_std::err;
use std::net::{TcpListener, TcpStream};
use crate::codec::{BinCodec, Codec, Codecs};
use crate::stub::ServerStub;
use std::io::Read;
use std::io::Write;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
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
    pub fn call(&self, stream: TcpStream) {
        self.stub.call(&self.handles, &self.codec, stream);
    }
}

pub trait Stub {
    fn accept(&self, arg: &[u8], codec: &Codecs) -> Result<Vec<u8>>;
}

pub trait Handler: Stub {
    type Req: DeserializeOwned;
    type Resp: Serialize;
    fn accept(&self, arg: &[u8], codec: &Codecs) -> Result<Vec<u8>> {
        //.or_else(|e| Result::Err(err!("{}",e)))?
        let req: Self::Req = codec.decode(arg)?;
        let data = self.handle(req)?;
        Ok(codec.encode(data)?)
    }
    fn handle(&self, req: Self::Req) -> Result<Self::Resp>;
}

impl<H: Handler> Stub for H {
    fn accept(&self, arg: &[u8], codec: &Codecs) -> Result<Vec<u8>> {
        <H as Handler>::accept(self, arg, codec)
    }
}

pub struct HandleFn<Req: DeserializeOwned, Resp: Serialize> {
    pub f: Box<dyn Fn(Req) -> Result<Resp>>,
}

impl<Req: DeserializeOwned, Resp: Serialize> Handler for HandleFn<Req, Resp> {
    type Req = Req;
    type Resp = Resp;

    fn handle(&self, req: Self::Req) -> dark_std::errors::Result<Self::Resp> {
        (self.f)(req)
    }
}

impl<Req: DeserializeOwned, Resp: Serialize> HandleFn<Req, Resp> {
    pub fn new<F: 'static>(f: F) -> Self where F: Fn(Req) -> Result<Resp> {
        Self {
            f: Box::new(f),
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
    pub fn register<H: 'static>(&mut self, name: &str, handle: H) where H: Stub {
        self.handles.insert(name.to_owned(), Box::new(handle));
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
    pub fn register_fn<Req: DeserializeOwned + 'static, Resp: Serialize + 'static, F: 'static>(&mut self, name: &str, f: F) where F: Fn(Req) -> Result<Resp> {
        self.handles.insert(name.to_owned(), Box::new(HandleFn::new(f)));
    }

    pub fn serve<A>(self, addr: A) where A: ToSocketAddrs {
        let listener = TcpListener::bind(addr).unwrap();
        println!(
            "Starting tcp echo server on {:?}",
            listener.local_addr().unwrap(),
        );
        let server = Arc::new(self);
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let server = server.clone();
                    co!(move || server.call(s));
                }
                Err(e) => error!("err = {:?}", e),
            }
        }
    }
}