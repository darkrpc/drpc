#![feature(test)]
extern crate test;
extern crate drpc;

use std::mem::MaybeUninit;
use std::thread::sleep;
use std::time::Duration;
use drpc::balance::{LoadBalance, LoadBalanceType, RpcClient};
use drpc::client::Client;

struct C{
   pub addr:String,
}
impl RpcClient for C{
    fn addr(&self) -> &str {
        &self.addr
    }
}
#[bench]
fn bench_balance(b: &mut test::Bencher) {
    let mut load =LoadBalance::<C>::new();
    load.put(C{addr:"127.0.0.1:13000".to_string()});
    load.put(C{addr:"127.0.0.1:13001".to_string()});
    load.put(C{addr:"127.0.0.1:13002".to_string()});
    load.put(C{addr:"127.0.0.1:13003".to_string()});
    b.iter(|| {
        load.do_balance(LoadBalanceType::Round, "");
    });
}