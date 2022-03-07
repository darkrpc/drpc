#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate redis;


use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use fast_log::sleep;
use drpc::{BalanceManger, RegistryCenter, ManagerConfig};
use drpc::server::Server;
use dark_std::errors::Result;
use redis::Commands;
use tokio::sync::Mutex;

pub struct RedisCenter {
    server_prefix: String,
    c: Mutex<redis::Client>,
}

impl RedisCenter {
    pub fn new() -> Self {
        Self {
            server_prefix: "service:".to_string(),
            c: Mutex::new(redis::Client::open("redis://127.0.0.1:6379".to_string()).expect("connect redis://127.0.0.1:6379")),
        }
    }
}

#[async_trait]
impl RegistryCenter for RedisCenter {
    async fn pull(&self) -> HashMap<String, Vec<String>> {
        let mut m = HashMap::new();
        let mut l = self.c.lock().await;
        if let Ok(v) = l.keys::<&str, Vec<String>>(&format!("{}*", self.server_prefix)) {
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
        return m;
    }

    async fn push(&self, service: String, addr: String, ex: Duration) -> Result<()> {
        let mut l = self.c.lock().await;
        l.hset::<String, String, String, ()>(format!("{}{}", self.server_prefix, &service), addr.to_string(), addr.to_string()).unwrap();
        l.expire::<String, ()>(format!("{}{}", &self.server_prefix, service), ex.as_secs() as usize);
        return Ok(());
    }
}

#[tokio::main]
async fn main() {
    let m = BalanceManger::new(ManagerConfig::default(), RedisCenter::new());
    let m_clone = m.clone();
    tokio::spawn(async move {
        spawn_server(m_clone).await;
    });
    sleep(Duration::from_secs(2));
    let m_clone = m.clone();
    tokio::spawn(async move {
        m_clone.spawn_pull().await;
    });
    sleep(Duration::from_secs(2));
    let r = m.call::<i32, i32>("test", "handle", 1).await;
    println!("-> test.handle(1)\n<- {}", r.unwrap());
}

async fn spawn_server(manager: Arc<BalanceManger>) {
    tokio::spawn(async move {
        manager.spawn_push("test".to_string(), "127.0.0.1:10000".to_string()).await;
    });
    let mut s = Server::default();
    s.register_fn("handle", |arg: i32| -> Result<i32>{
        Ok(1)
    });
    s.serve("127.0.0.1:10000");
}