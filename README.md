# drpc

Drpc - Correct, high performance, robust, easy use Remote invocation framework

<img style="width: 100px;height: 98px;border-radius:20px;" width="100" height="100" src="logo.png" />

drpc

* Super high performance, double performance(qps) as fast as Tarpc (Google)
* based T-L-V.for example:  ```[Tag][Length][Value]```
* support Custom Serialization crate. for example: bincode,json,bson...any [serde](https://serde.rs/) Serialization
* support Load Balance.(Round/Random/Hash/MinConnect)
* support Custom registry, microservices. see [redis_registry](example/src/redis_registry.rs)
* support tokio，this is async/await crate
* zero overhead, Accept/Response only serialize the once and deserialization once

T-L-V layout

```rust
// Frame layout
// id(u64) + ok(u8) + len(u64) + payload([u8; len])

// request frame layout. payload = method([u8;len])+'\n'(u8)+arg_data([u8;len])
// id(u64) + ok(u8) + len(u64) + payload([u8; len])

// response frame layout.ok=0? payload = error string,ok=1? payload = data
// id(u64) + ok(u8) + len(u64) + payload ([u8; len])

// Header Length layout
// head(8(id)+1(ok)+8(length)=17)
```

## qps benchmark-  remote_method(i32)->i32
| Framework   | Platform(1-server-1-client) |  ns/operation（lower is better） | Qps(higher is better) |
|-------------|-----------------------------|------ |------ |
| drpc/tokio  | AMD 5950x-16 CPU, 32G mem   |  49213 ns/op   |  20317 QPS/s  |
| tarpc/tokio | AMD 5950x-16 CPU, 32G mem   |  105644 ns/op  |  9465 QPS/s  |



## how to use?

```toml
tokio = { version = "1", features = ["full"] }
dark-std = "0.1"
drpc = "0.1"
```

* client

```rust
use drpc::client::Client;
use drpc::codec::BinCodec;
let c = Client::<BinCodec>::dial("127.0.0.1:10000").await.unwrap();
let resp:i32 = c.call("handle", 1).await.unwrap();
println!("resp=>>>>>>>>>>>>>> :{}", resp);
```

* server

```rust
use drpc::server::Server;
use drpc::Result;
use drpc::codec::BinCodec;
async fn handle(req: i32) -> Result<i32> {
    Ok(req)
}
let mut s =  Server::<BinCodec>::new();
s.register_fn("handle", handle);
s.register_fn("handle2", | arg:i32| async move{
Ok(arg + 1)
});
s.serve("0.0.0.0:10000").await;
```
