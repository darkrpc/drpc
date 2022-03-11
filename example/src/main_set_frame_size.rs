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
use serde::Serialize;
use drpc::client::Client;
use drpc::codec::{Codecs, JsonCodec};
use drpc::server::{Handler, Server, Stub};
use drpc::Result;
use tokio::time::sleep;

pub async fn handle(req: String) -> drpc::Result<String> {
    Ok(req)
}

#[tokio::main]
async fn main() {
    fast_log::init(Config::new().console());

    drpc::set_frame_len(10 * 1024 * 1024);//10MB

    let mut msg = "".to_string();
    for i in 0..10000 {
        msg.push_str(&i.to_string());
    }
    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        let c = Client::dial("127.0.0.1:10000").await.unwrap();
        println!("dial success");
        let resp: String = c.call("handle", msg).await.unwrap();
        println!("resp=>>>>>>>>>>>>>> :{}", resp);
        exit(0);
    });
    let mut s = Server::default();
    s.register_fn("handle", handle);
    s.serve("0.0.0.0:10000").await;
    println!("Hello, world!");
}
