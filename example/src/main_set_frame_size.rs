use std::process::exit;
use std::time::Duration;
use fast_log::config::Config;
use tokio::time::sleep;
use drpc::client::Client;
use drpc::codec::BinCodec;
use drpc::server::Server;

pub async fn handle(req: String) -> drpc::Result<String> {
    Ok(req)
}

#[tokio::main]
async fn main() {
    fast_log::init(Config::new().console()).expect("fast_log init fail");

    drpc::set_frame_len(16 * 1024 * 1024);//16MB

    let mut msg = "".to_string();
    for i in 0..10000 {
        msg.push_str(&i.to_string());
    }
    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        let c = Client::<BinCodec>::dial("127.0.0.1:10000").await.unwrap();
        println!("dial success");
        let resp: String = c.call("handle", msg).await.unwrap();
        println!("resp=>>>>>>>>>>>>>> :{}", resp);
        exit(0);
    });
    let mut s = Server::default();
    s.register_fn("handle", handle);
    s.serve("0.0.0.0:10000").await;
}
