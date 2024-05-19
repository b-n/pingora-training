# Load balance

Add a simple load balancer to the server 

## 1. Add/Change dependencies

- Add async-trait
- Add `lb` feature to pingora

```toml
[dependencies]
pingora = { version = "0.2", features = ["lb"] }
async-trait = { version = "0.1" }
```

## 2. Create a load balancing service

- The service will return an upstream peer
- The peer is retrieved from the loadbalancer which we will select using roundrobin

The required imports:

```rs
use async_trait::async_trait;
use pingora::lb::{selection::RoundRobin, LoadBalancer};
use pingora::proxy::{http_proxy_service, ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora::Result;
use std::sync::Arc;
```

Our service struct:

```rs
pub struct LBService {
    balancer: Arc<LoadBalancer<RoundRobin>>,
    name: String,
}
```

The peer selection, exposing the load balancer as a proxy service:

```rs
#[async_trait]
impl ProxyHttp for LBService {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self.balancer.select(b"", 256).unwrap();

        println!("Selected upstream: {:?}", upstream);

        let peer = Box::new(HttpPeer::new(upstream, true, self.name.clone()));

        Ok(peer)
    }
}
```

👆 The ProxyHttp impl is needed to turn the LoadBalancer into a service. It's main use it to select a peer from the inner Loadbalancer.

Note: We do nothing with the incoming request (`Session`) or the context object at this stage.

## 3. Add the service to the server

```rs
fn main() {
    // ..
    let upstreams = {
        let upstreams = LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).unwrap();
        Arc::new(upstreams)
    }

    let proxy_service = {
        let service = LBService {
            name: "one.one.one.one".to_string(),
            balancer: upstreams,
        };
        let mut proxy = http_proxy_service(&server.configuration, service);
        proxy.add_tcp("0.0.0.0:8001");
        proxy
    };

    server.add_service(proxy_service);

    server.run_forever();
}
```

Block scoping the upstream/service is here just to clarify for later use.

## 4. Run our proxy

```sh
cargo run 
```

And curl a couple of times in a terminal somewhere:

```sh
curl http://localhost:8001/
```

The responses should be 403's, but the proxy should be outputing something similar to the following:

```
Selected upstream: Backend { addr: Inet(1.0.0.1:443), weight: 1 }
Selected upstream: Backend { addr: Inet(1.1.1.1:443), weight: 1 }
Selected upstream: Backend { addr: Inet(1.0.0.1:443), weight: 1 }
Selected upstream: Backend { addr: Inet(1.1.1.1:443), weight: 1 }
```

👆 As you can see, the server being selected is cycling between each upstream every time we make a call (Round robin selection).

## 5. Load test it with `oha`

⚠ You are going to create some load on the cloudflare dns servers (it's probably insignificant to them, but still important to know just the same).

If you don't have `oha`, you can install it with `cargo install oha`.

```sh
oha -z 1s http://localhost:8001
```
