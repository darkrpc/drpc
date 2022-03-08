# drpc

drpc

* based T-L-V.for example:  ```[Tag][Length][Value]```
* support json/bincode
* support load balance(Round/Random/Hash/MinConnect)
* support Custom registry, microservices. see [redis_registry](example/src/redis_registry.rs)
* support tokio

## how to use?

```toml
tokio = { version = "1", features = ["full"] }
dark-std="0.1"
drpc = "0.1"
```

* client

```rust
use drpc::client::Client;
let c = Client::dial("127.0.0.1:10000").await.unwrap();
let resp:i32 = c.call("handle", 1).await.unwrap();
println!("resp=>>>>>>>>>>>>>> :{}", resp);
```

* server

```rust
use drpc::server::Server;
use dark_std::errors::Result;
async fn handle(req: i32) -> Result<i32> {
    Ok(req)
}
let mut s = Server::default();
s.register_fn("handle", handle).await;
s.serve("0.0.0.0:10000").await;
```