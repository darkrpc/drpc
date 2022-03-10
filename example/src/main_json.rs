use std::future::Future;
use std::io::Sink;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::pin::Pin;
use std::process::exit;
use std::time::Duration;
use dark_std::err;
use fast_log::config::Config;
use fast_log::filter::ModuleFilter;
use serde::de::DeserializeOwned;
use drpc::client::Client;
use drpc::codec::{Codecs, JsonCodec};
use drpc::server::{Handler, Server, Stub};
use dark_std::errors::Result;
use futures::future::BoxFuture;
use tokio::time::sleep;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize,Debug)]
pub struct DTO {
    pub name: String,
    pub age: i32,
}

pub async fn handle(mut req: DTO) -> drpc::Result<DTO> {
    println!("recv dto<<<<<<<<<<<<<={:?}", req);
    req.name = "ye,you is joe".to_string();
    Ok(req)
}

#[tokio::main]
async fn main() {
    fast_log::init(Config::new().console());
    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        let mut c = Client::dial("127.0.0.1:10000").await.unwrap();
        c.codec = Codecs::JsonCodec(JsonCodec {});
        println!("dial success");
        let resp: DTO = c.call("handle", DTO {
            name: "joe".to_string(),
            age: 18,
        }).await.unwrap();
        println!("resp=>>>>>>>>>>>>>> :{:?}", resp);
        exit(0);
    });
    let mut s = Server::default();
    s.codec = Codecs::JsonCodec(JsonCodec {});
    s.register_fn("handle", handle);
    s.serve("0.0.0.0:10000").await;
    println!("Hello, world!");
}
