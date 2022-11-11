use dark_std::err;
use dark_std::errors::Result;
use dark_std::sync::SyncHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::balance::{LoadBalance, LoadBalanceType};
use crate::client::Client;
use crate::codec::Codec;

/// To fetch remote service addr list
#[async_trait]
pub trait RegistryCenter: Sync + Send {
    /// Fetch [service]Vec<addr>
    async fn pull(&self) -> HashMap<String, Vec<String>>;
    async fn push(&self, service: String, addr: String, ex: Duration) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ManagerConfig {
    pub balance: LoadBalanceType,
    pub interval: Duration,
}

impl ManagerConfig {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn balance(mut self, balance: LoadBalanceType) -> Self {
        self.balance = balance;
        self
    }
    pub fn interval(mut self, d: Duration) -> Self {
        self.interval = d;
        self
    }
}

impl Default for ManagerConfig {
    fn default() -> Self {
        ManagerConfig {
            balance: LoadBalanceType::Round,
            interval: Duration::from_secs(5),
        }
    }
}

/// A connect manager that accepts a server addresses and make a client list.
pub struct BalanceManger<C: Codec> {
    pub config: ManagerConfig,
    pub clients: SyncHashMap<String, LoadBalance<Client<C>>>,
    pub fetcher: Arc<dyn RegistryCenter>,
}

impl<C: Codec> BalanceManger<C> {
    pub fn new<F>(cfg: ManagerConfig, f: F) -> Arc<Self>
    where
        F: RegistryCenter + 'static,
    {
        Arc::new(Self {
            config: cfg,
            clients: SyncHashMap::new(),
            fetcher: Arc::new(f),
        })
    }

    /// pull addr list once
    pub async fn pull(&self) -> Result<()> {
        let addrs = self.fetcher.pull().await;
        if addrs.is_empty() {
            self.clients.clear();
        }
        for (s, addrs) in addrs {
            let balance = self.clients.get(&s);
            if let Some(clients) = balance {
                for addr in &addrs {
                    if !clients.contains(addr) {
                        let c = Client::dial(addr).await?;
                        clients.put(c);
                    }
                }
                let mut removes = vec![];
                for x in &clients.rpc_clients {
                    if !addrs.contains(&x.addr) {
                        removes.push(&x.addr);
                    }
                }
                for x in removes {
                    if let Some(client) = clients.remove(x) {
                        if Arc::strong_count(&client) <= 1 {
                            if let Ok(mut client) = Arc::try_unwrap(client) {
                                client.shutdown().await;
                            }
                        }
                    }
                }
            } else {
                let clients = LoadBalance::new();
                for x in addrs {
                    let c = Client::dial(&x).await?;
                    clients.put(c);
                }
                self.clients.insert(s, clients);
            }
        }
        return Ok(());
    }

    /// Spawn an loop pull
    pub async fn spawn_pull(&self) {
        loop {
            let r = self.pull().await;
            if r.is_err() {
                log::error!("service fetch fail:{}", r.err().unwrap());
            }
            sleep(self.config.interval).await;
        }
    }

    /// Push addr into register once
    pub async fn push(&self, service: String, addr: String) -> Result<()> {
        self.fetcher
            .push(
                service.clone(),
                addr.clone(),
                self.config.interval.clone() * 2,
            )
            .await
    }

    /// Spawn an loop push
    pub async fn spawn_push(&self, service: String, addr: String) {
        loop {
            let r = self.push(service.clone(), addr.clone()).await;
            if r.is_err() {
                log::error!("service fetch fail:{}", r.err().unwrap());
            }
            sleep(self.config.interval).await;
        }
    }

    pub async fn call<Arg, Resp>(&self, service: &str, func: &str, arg: Arg) -> Result<Resp>
    where
        Arg: Serialize,
        Resp: DeserializeOwned,
    {
        return match self
            .clients
            .get(service)
            .ok_or(err!("no service '{}' find!", service))?
            .do_balance(self.config.balance, service)
        {
            None => Err(err!("no service '{}' find!", service)),
            Some(c) => c.call(func, arg).await,
        };
    }
}
