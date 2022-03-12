# drpc

drpc - Correct, high performance, robust, easy use,

<img style="width: 100px;height: 98px;border-radius:20px;" src="logo.png" />

drpc

* Super high performance,Twice the performance(qps) as fast as Tarpc (Google)
* based T-L-V.for example:  ```[Tag][Length][Value]```
* support json/bincode
* support load balance(Round/Random/Hash/MinConnect)
* support Custom registry, microservices. see [redis_registry](example/src/redis_registry.rs)
* support tokio，this is async/await crate
* Accept/Response only serialize the once and deserialization once

T-L-V layout

```rust
// Frame layout
// id(u64) + ok(u8) + len(u64) + payload([u8; len])

// request frame layout. payload = method(string)+'\n'+arg_data
// id(u64) + ok(u8) + len(u64) + payload([u8; len])

// response frame layout.ok=0? payload = error string,ok=1? payload = data
// id(u64) + ok(u8) + len(u64) + payload ([u8; len])
```

## how to use?

```toml
tokio = { version = "1", features = ["full"] }
dark-std = "0.1"
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
use drpc::Result;

async fn handle(req: i32) -> Result<i32> {
    Ok(req)
}
let mut s = Server::default ();
s.register_fn("handle", handle);
s.register_fn("handle2", | arg:i32| async move{
Ok(arg + 1)
});
s.serve("0.0.0.0:10000").await;
```