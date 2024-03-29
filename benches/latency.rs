#![feature(test)]
#[cfg(test)]
extern crate test;

use drpc::client::Client;
use drpc::codec::BinCodec;
use drpc::server::Server;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;
use test::Bencher;

#[cfg(test)]
#[bench]
fn latency(bencher: &mut Bencher) {
    let rt_server = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt_server.spawn(async {
        pub async fn handle(req: i32) -> dark_std::errors::Result<i32> {
            Ok(req + 1)
        }
        let mut s = Server::default();
        s.register_fn("handle", handle);
        s.serve(("127.0.0.1", 10000)).await;
        println!("rpc served");
    });
    let (s, r) = channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            sleep(Duration::from_secs(1));
            let c = Client::<BinCodec>::dial("127.0.0.1:10000").await.unwrap();
            println!("dial success");
            let resp: i32 = c.call("handle", 1).await.unwrap();
            println!("resp=>>>>>>>>>>>>>> :{}", resp);
            loop {
                match r.recv() {
                    Ok(v) => {
                        let _resp: i32 = c.call("handle", v).await.unwrap();
                    }
                    Err(_e) => {}
                }
            }
        });
    });
    s.send(1).expect("panic send");
    sleep(Duration::from_secs(2));
    bencher.iter(|| {
        s.send(1).expect("panic send");
    });
}
