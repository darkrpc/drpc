use std::process::exit;
use std::time::Duration;
use dark_std::err;
use dark_std::errors::Error;
use fast_log::config::Config;
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use drpc::client::Client;
use drpc::codec::Codec;
use drpc::server::Server;

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
        let c = Client::<BsonCodec>::dial("127.0.0.1:10000").await.unwrap();
        println!("dial success");
        let resp: DTO = c.call("handle", DTO {
            name: "joe".to_string(),
            age: 18,
        }).await.unwrap();
        println!("resp=>>>>>>>>>>>>>> :{:?}", resp);
        exit(0);
    });
    let mut s = Server::<BsonCodec>::new();
    s.register_fn("handle", handle);
    s.serve("0.0.0.0:10000").await;
}

#[derive(Clone,Debug,Default)]
pub struct BsonCodec{}

impl Codec for BsonCodec{
    fn encode<T: Serialize>(&self, arg: T) -> std::result::Result<Vec<u8>, Error> {
        bson::to_vec(&arg).map_err(|e|err!("{}",e.to_string()))
    }

    fn decode<T: DeserializeOwned>(&self, arg: &[u8]) -> std::result::Result<T, Error> {
        bson::from_slice(arg).map_err(|e|err!("{}",e.to_string()))
    }
}