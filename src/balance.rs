use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use dark_std::sync::SyncVec;

///Defines the minimum abstraction required by the load algorithm
///The four common load algorithms simply provide remote IP addresses
///To use the LoadBalance structure, the client must implement this trait
pub trait RpcClient {
    fn addr(&self) -> &str;
}

#[derive(Debug)]
pub struct LoadBalance<C> where C: RpcClient {
    pub index: AtomicUsize,
    pub rpc_clients: SyncVec<Arc<C>>,
}

/// an load balance type.
#[derive(Clone, Debug, Copy)]
pub enum LoadBalanceType {
    /// RPC clients take turns to execute
    Round,
    /// RPC clients random pick one
    Random,
    /// RPC clients pick one by address's hashcode，so client_ip with client that is matches in pairs
    Hash,
    /// RPC clients pick on by Has the minimum number of TCP connections
    MinConnect,
}

impl<C> LoadBalance<C> where C: RpcClient {
    pub fn new() -> Self {
        Self {
            index: AtomicUsize::new(0),
            rpc_clients: SyncVec::new(),
        }
    }

    /// put client,and return old client
    pub async fn put(&self, arg: C) -> Option<Arc<C>> {
        let arg = Some(Arc::new(arg));
        let addr = arg.as_deref().unwrap().addr();
        let mut idx = 0;
        for x in &self.rpc_clients {
            if x.addr().eq(addr) {
                let rm = self.rpc_clients.remove(idx).await;
                if rm.is_none() {
                    self.rpc_clients.push(arg.unwrap()).await;
                    return None;
                }
                return rm;
            }
            idx += 1;
        }
        if let Some(arg) = arg {
            self.rpc_clients.push(arg).await;
        }
        return None;
    }

    pub async fn remove(&self, address: &str) -> Option<Arc<C>> {
        let mut idx = 0;
        for x in &self.rpc_clients {
            if x.addr().eq(address) {
                return self.rpc_clients.remove(idx).await;
            }
            idx += 1;
        }
        return None;
    }

    pub fn contains(&self, address: &str) -> bool {
        for x in &self.rpc_clients {
            if x.addr().eq(address) {
                return true;
            }
        }
        return false;
    }

    pub async fn clear(&self) {
        self.rpc_clients.clear().await;
    }

    pub fn do_balance(&self, b: LoadBalanceType, from: &str) -> Option<Arc<C>> {
        match b {
            LoadBalanceType::Round => {
                self.round_pick_client()
            }
            LoadBalanceType::Random => {
                self.random_pick_client()
            }
            LoadBalanceType::Hash => {
                self.hash_pick_client(from)
            }
            LoadBalanceType::MinConnect => {
                self.min_connect_client()
            }
        }
    }

    fn hash_pick_client(&self, from: &str) -> Option<Arc<C>> {
        let length = self.rpc_clients.len() as i64;
        if length == 0 {
            return None;
        }
        let def_key: String;
        if from.is_empty() {
            def_key = format!("{}", rand::random::<i32>());
        } else {
            def_key = from.to_string();
        }
        let hash = {
            let mut value = 0i64;
            let mut i = 0;
            for x in def_key.as_bytes() {
                i += 1;
                value += (*x as i64) * i;
            }
            value
        };
        return Some(self.rpc_clients.get((hash % length) as usize).unwrap().clone());
    }

    fn random_pick_client(&self) -> Option<Arc<C>> {
        let length = self.rpc_clients.len();
        if length == 0 {
            return None;
        }
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let rand_index: usize = rng.gen_range(0..length);
        if rand_index < length {
            return Some(self.rpc_clients.get(rand_index).unwrap().clone());
        }
        return None;
    }

    fn round_pick_client(&self) -> Option<Arc<C>> {
        let length = self.rpc_clients.len();
        if length == 0 {
            return None;
        }
        let idx = self.index.load(Ordering::SeqCst);
        if (idx + 1) >= length {
            self.index.store(0, Ordering::SeqCst)
        } else {
            self.index.store(idx + 1, Ordering::SeqCst);
        }
        let return_obj = self.rpc_clients.get(idx).unwrap().clone();
        return Some(return_obj);
    }

    fn min_connect_client(&self) -> Option<Arc<C>> {
        let mut min = -1i64;
        let mut result = None;
        for x in &self.rpc_clients {
            if min == -1 || Arc::strong_count(x) < min as usize {
                min = Arc::strong_count(x) as i64;
                result = Some(x.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod test {
    use crate::balance::{RpcClient, LoadBalance, LoadBalanceType};

    impl RpcClient for String {
        fn addr(&self) -> &str {
            &self
        }
    }

    #[tokio::test]
    async fn test_put() {
        let mut load: LoadBalance<String> = LoadBalance::new();
        load.put("127.0.0.1:13000".to_string()).await;
        load.put("127.0.0.1:13001".to_string()).await;

        let old = load.put("127.0.0.1:13001".to_string()).await.unwrap();
        assert_eq!(old.addr(), "127.0.0.1:13001".to_string());
    }

    #[tokio::test]
    async fn test_remove() {
        let mut load: LoadBalance<String> = LoadBalance::new();
        load.put("127.0.0.1:13000".to_string()).await;
        load.put("127.0.0.1:13001".to_string()).await;

        let old = load.remove("127.0.0.1:13000").await.unwrap();
        assert_eq!(old.addr(), "127.0.0.1:13000".to_string());
    }

    #[tokio::test]
    async fn test_min_connect() {
        let mut load: LoadBalance<String> = LoadBalance::new();
        load.put("127.0.0.1:13000".to_string()).await;
        load.put("127.0.0.1:13001".to_string()).await;
        load.put("127.0.0.1:13002".to_string()).await;
        load.put("127.0.0.1:13003".to_string()).await;
        let mut v = vec![];
        let item = load.do_balance(LoadBalanceType::MinConnect, "");
        println!("select:{}", item.as_ref().unwrap().addr());
        v.push(item);
        let item = load.do_balance(LoadBalanceType::MinConnect, "");
        println!("select:{}", item.as_ref().unwrap().addr());
        v.push(item);
        let item = load.do_balance(LoadBalanceType::MinConnect, "");
        println!("select:{}", item.as_ref().unwrap().addr());
        v.push(item);
        let item = load.do_balance(LoadBalanceType::MinConnect, "");
        println!("select:{}", item.as_ref().unwrap().addr());
        v.push(item);
        let item = load.do_balance(LoadBalanceType::MinConnect, "");
        println!("select:{}", item.as_ref().unwrap().addr());
        v.push(item);
        let item = load.do_balance(LoadBalanceType::MinConnect, "");
        println!("select:{}", item.as_ref().unwrap().addr());
        v.push(item);
    }
}