# Prometheus metrics

Pingora has a helper to construct a prometheus metrics service.

## 1. Add a metric for request counter per upstream address

Add the following dependencies:

```toml
prometheus = { version = "0.13.4" }
lazy_static = { version = "1.4.0" }
```

And import into the code:

```rs
use pingora::services::listening::Service;
use prometheus::{self, IntCounterVec};
use lazy_static::lazy_static;
```

Define a metric. In the below example, we create an integer request counter with a label called "upstream"

```rs
lazy_static! {
    static ref REQUESTS: IntCounterVec = prometheus::register_int_counter_vec!(
        "requests_total",
        "Total number of requests.",
        &["upstream"]
    )
    .unwrap();
}
```

Utilise the metric counter in the upstream peer selector from the load balancer:

```rs
    // ..
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self.balancer.select(b"", 256).unwrap();

        REQUESTS
            .with_label_values(&[&format!("{}", upstream.addr)])
            .inc();

        let peer = Box::new(HttpPeer::new(upstream, true, self.name.clone()));

        Ok(peer)
    }
    // ..
```

## 2. Add the prometheus service

```rs
    // ..
    let prom_service = {
        let mut service = Service::prometheus_http_service();
        service.add_tcp("0.0.0.0:9090");
        service
    };
    server.add_service(prom_service);

    server.bootstrap();
    server.run_forever();
    // ..
```

## 3. Test it

```sh
$ cargo run
```

Curl the load blanacer a couple of times first:

```sh
$ curl http://localhost:8001
$ curl http://localhost:8001
$ curl http://localhost:8001
```

And now the result should look as follows:

```sh
$ curl http://localhost:9090
# HELP requests_total Total number of requests.
# TYPE requests_total counter
requests_total{upstream="1.0.0.1:443"} 2
requests_total{upstream="1.1.1.1:443"} 1
```

Note: the metrics endpoint will show nothing until a metric is first counted.
