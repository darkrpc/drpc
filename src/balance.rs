use dark_std::sync::SyncVec;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Defines the minimum abstraction required by the load algorithm.
/// The four common load algorithms simply provide remote IP addresses
/// to use the LoadBalance structure. The client must implement this trait.
pub trait RpcClient {
    fn addr(&self) -> &str;
}

#[derive(Debug)]
pub struct LoadBalance<C>
where
    C: RpcClient,
{
    pub index: AtomicUsize,
    pub rpc_clients: SyncVec<Arc<C>>,
}

/// A load balance type.
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

impl<C> LoadBalance<C>
where
    C: RpcClient,
{
    pub fn new() -> Self {
        Self {
            index: AtomicUsize::new(0),
            rpc_clients: SyncVec::new(),
        }
    }

    /// Put a client and return the old one.
    pub fn put(&self, arg: C) -> Option<Arc<C>> {
        let arg = Some(Arc::new(arg));
        let addr = arg.as_deref().unwrap().addr();
        let mut idx = 0;
        for x in &self.rpc_clients {
            if x.addr().eq(addr) {
                let rm = self.rpc_clients.remove(idx);
                if rm.is_none() {
                    self.rpc_clients.push(arg.unwrap());
                    return None;
                }
                return rm;
            }
            idx += 1;
        }
        if let Some(arg) = arg {
            self.rpc_clients.push(arg);
        }
        return None;
    }

    pub fn remove(&self, address: &str) -> Option<Arc<C>> {
        let mut idx = 0;
        for x in &self.rpc_clients {
            if x.addr().eq(address) {
                return self.rpc_clients.remove(idx);
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

    pub fn clear(&self) {
        self.rpc_clients.clear();
    }

    pub fn do_balance(&self, b: LoadBalanceType, from: &str) -> Option<Arc<C>> {
        match b {
            LoadBalanceType::Round => self.round_pick_client(),
            LoadBalanceType::Random => self.random_pick_client(),
            LoadBalanceType::Hash => self.hash_pick_client(from),
            LoadBalanceType::MinConnect => self.min_connect_client(),
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
        return Some(
            self.rpc_clients
                .get((hash % length) as usize)
                .unwrap()
                .clone(),
        );
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
