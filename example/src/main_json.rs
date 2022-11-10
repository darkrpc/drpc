use drpc::client::Client;
use drpc::codec::JsonCodec;
use drpc::server::Server;
use fast_log::config::Config;
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Serialize, Deserialize, Debug)]
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
    fast_log::init(Config::new().console()).expect("fast_log init fail");
    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        let c = Client::<JsonCodec>::dial("127.0.0.1:10000").await.unwrap();
        println!("dial success");
        let resp: DTO = c
            .call(
                "handle",
                DTO {
                    name: "joe".to_string(),
                    age: 18,
                },
            )
            .await
            .unwrap();
        println!("resp=>>>>>>>>>>>>>> :{:?}", resp);
        exit(0);
    });
    let mut s = Server::<JsonCodec>::new();
    s.register_fn("handle", handle);
    s.serve("0.0.0.0:10000").await;
}
