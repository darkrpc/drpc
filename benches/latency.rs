#![feature(test)]
#[macro_use]
extern crate drpc;

#[cfg(test)]
extern crate test;
#[macro_use]
extern crate tokio;

use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;
use test::Bencher;
use drpc::client::Client;
use drpc::codec::BinCodec;
use drpc::server::Server;


#[cfg(test)]
#[bench]
fn latency(bencher: &mut Bencher) {
    let rt_server = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build().unwrap();
    rt_server.spawn(async {
        pub async fn handle(req: i32) -> dark_std::errors::Result<i32> {
            Ok(req + 1)
        }
        let mut s = Server::default();
        s.register_fn("handle", handle);
        s.serve(("127.0.0.1", 10000)).await;
        println!("rpc served");
    });
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build().unwrap();
    let c = rt.block_on(async {
        sleep(Duration::from_secs(1));
        let mut c = Client::<BinCodec>::dial("127.0.0.1:10000").await.unwrap();
        println!("dial success");
        let resp: i32 = c.call("handle", 1).await.unwrap();
        println!("resp=>>>>>>>>>>>>>> :{}", resp);
        c
    });
    let (s, r) = channel();
    std::thread::spawn(move ||{
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build().unwrap();
        rt.block_on(async {
            loop {
                match r.recv() {
                    Ok(v) => {
                        let resp: i32 = c.call("handle", v).await.unwrap();
                    }
                    Err(e) => {}
                }
            }
        });
    });
    sleep(Duration::from_secs(1));
    bencher.iter(|| {
        s.send(1).expect("panic send");
    });
}
