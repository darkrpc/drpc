use std::process::exit;
use std::time::Duration;
use fast_log::config::Config;
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use drpc::client::Client;
use drpc::codec::BinCodec;
use drpc::server::Server;

pub async fn handle(req: i32) -> drpc::Result<i32> {
    Ok(req + 1)
}

#[tokio::main]
async fn main() {
    fast_log::init(Config::new().console()).expect("fast_log init fail");
    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        let c = Client::<BinCodec>::dial("127.0.0.1:10000").await.unwrap();
        println!("dial success");
        let resp: i32 = c.call("handle", 1).await.unwrap();
        println!("resp=>>>>>>>>>>>>>> :{}", resp);
        exit(0);
    });
    let mut s = Server::<BinCodec>::new();
    s.register_fn("handle", handle);
    s.serve("0.0.0.0:10000").await;
}
