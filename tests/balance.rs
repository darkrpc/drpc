#[cfg(test)]
mod test {
    use drpc::balance::{LoadBalance, LoadBalanceType, RpcClient};

    pub struct MockClient {
        pub addr: String,
    }
    impl RpcClient for MockClient {
        fn addr(&self) -> &str {
            &self.addr
        }
    }
    impl From<&str> for MockClient {
        fn from(value: &str) -> Self {
            MockClient {
                addr: value.to_string(),
            }
        }
    }

    #[tokio::test]
    async fn test_put() {
        let load: LoadBalance<MockClient> = LoadBalance::new();
        load.put("127.0.0.1:13000".into());
        load.put("127.0.0.1:13001".into());

        let old = load.put("127.0.0.1:13001".into()).unwrap();
        assert_eq!(old.addr(), "127.0.0.1:13001".to_string());
    }

    #[tokio::test]
    async fn test_remove() {
        let load: LoadBalance<MockClient> = LoadBalance::new();
        load.put("127.0.0.1:13000".into());
        load.put("127.0.0.1:13001".into());

        let old = load.remove("127.0.0.1:13000").unwrap();
        assert_eq!(old.addr(), "127.0.0.1:13000".to_string());
    }

    #[tokio::test]
    async fn test_min_connect() {
        let load: LoadBalance<MockClient> = LoadBalance::new();
        load.put("127.0.0.1:13000".into());
        load.put("127.0.0.1:13001".into());
        load.put("127.0.0.1:13002".into());
        load.put("127.0.0.1:13003".into());
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
