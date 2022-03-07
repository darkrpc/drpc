use std::io::Sink;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use fast_log::config::Config;
use fast_log::filter::ModuleFilter;
use serde::de::DeserializeOwned;
use serde::Serialize;
use drpc::client::Client;
use drpc::codec::{Codecs, JsonCodec};
use drpc::server::{Handler, Server, Stub};
use dark_std::errors::Result;
use tokio::time::sleep;

fn handle(req: i32) -> dark_std::errors::Result<i32> {
    return Ok(req + 1);
}

fn main() {
    fast_log::init(Config::new()
        .console()
        .filter(ModuleFilter::new_exclude(vec!["drpc::".to_string()])));
     tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        let c = Client::dial("127.0.0.1:10000").unwrap();
        //c.codec = Codecs::JsonCodec(JsonCodec{});
        println!("dial success");
        let resp:i32 = c.call("handle",1).unwrap();
        println!("resp=>>>>>>>>>>>>>> :{}",resp);
        exit(0);
    });
    let mut s = Server::default();
    //s.codec = Codecs::JsonCodec(JsonCodec{});
    s.register_fn("handle", handle).await;
    s.register_fn("handle_fn2", |arg:i32| -> Result<i32>{
        Ok(1)
    }).await;
    s.serve("0.0.0.0:10000").await;
    println!("Hello, world!");
}
