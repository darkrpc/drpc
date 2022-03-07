use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use fast_log::sleep;
use mco::{co};
use drpc::{BalanceManger, RegistryCenter, ManagerConfig};
use drpc::server::Server;
use dark_std::errors::Result;
use dark_std::sync::Mutex;
use mco_redis_rs::Commands;

pub struct RedisCenter {
    server_prefix: String,
    c: Mutex<mco_redis_rs::Client>,
}

impl RedisCenter {
    pub fn new() -> Self {
        Self {
            server_prefix: "service:".to_string(),
            c: Mutex::new(mco_redis_rs::Client::open("redis://127.0.0.1:6379".to_string()).expect("connect redis://127.0.0.1:6379")),
        }
    }
}

impl RegistryCenter for RedisCenter {
    fn pull(&self) -> HashMap<String, Vec<String>> {
        let mut m = HashMap::new();
        if let Ok(mut l) = self.c.lock() {
            if let Ok(v) = l.keys::<&str, Vec<String>>(&format!("{}*",self.server_prefix)) {
                for service in v {
                    if let Ok(list) = l.hgetall::<&str, HashMap<String, String>>(service.as_str()) {
                        let mut data = Vec::with_capacity(list.len());
                        for (k, _) in list {
                            data.push(k);
                        }
                        m.insert(service.trim_start_matches(&self.server_prefix).to_string(), data);
                    }
                }
            }
        }
        return m;
    }

    fn push(&self, service: String, addr: String, ex: Duration) -> Result<()> {
        if let Ok(mut l) = self.c.lock() {
            l.hset::<String, String, String, ()>(format!("{}{}", self.server_prefix, &service), addr.to_string(), addr.to_string()).unwrap();
            l.expire::<String, ()>(format!("{}{}", &self.server_prefix, service), ex.as_secs() as usize);
        }
        return Ok(());
    }
}

fn main() {
    let m = BalanceManger::new(ManagerConfig::default(), RedisCenter::new());
    let m_clone = m.clone();
    co!(|| {
        spawn_server(m_clone);
    });
    sleep(Duration::from_secs(2));
    let m_clone = m.clone();
    co!(0x2000,move ||{
       m_clone.spawn_pull();
    });
    sleep(Duration::from_secs(2));
    let r = m.call::<i32, i32>("test", "handle", 1);
    println!("-> test.handle(1)\n<- {}", r.unwrap());
}

fn spawn_server(manager: Arc<BalanceManger>) {
    co!(0x2000,move ||{
         manager.spawn_push("test".to_string(), "127.0.0.1:10000".to_string());
    });
    let mut s = Server::default();
    s.register_fn("handle", |arg: i32| -> Result<i32>{
        Ok(1)
    });
    s.serve("127.0.0.1:10000");
}