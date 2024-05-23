# Gateway

Load balancing can only achieve so much. Let's route.

Let's also keep metrics of where we sent the request.

## 1. Handle multiple LoadBalancers

Summary:

- We've only constructed one LoadBalancer
- We want to use pingora as a gateway to multiple services
- We are assuming we still need to load balancer to the upstreams

Firstly, we'll create a helper function to generate our health checked load balancers.

New import:

```rs
use pingora::lb::selection::{BackendIter, BackendSelection};
use pingora::services::background::GenBackgroundService;
```

Create a helper function which returns the health check background task, and the resultant upstreams:

```rs
fn build_lb_service<S>(
    upstreams: &[&str],
) -> (GenBackgroundService<LoadBalancer<S>>, Arc<LoadBalancer<S>>)
where
    S: BackendSelection + 'static,
    S::Iter: BackendIter,
{
    let mut upstreams = LoadBalancer::try_from_iter(upstreams).unwrap();

    let health_check = TcpHealthCheck::new();
    upstreams.set_health_check(health_check);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let health_check = background_service("health check", upstreams);
    let health_checked_upstreams = health_check.task();
    (health_check, health_checked_upstreams)
}
```

Use our new helper function:

```diff
-    let (health_check, upstreams) = {
-        let mut upstreams: LoadBalancer<_> =
-            LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443", "127.0.0.1:443"]).unwrap();
-
-        let health_check = TcpHealthCheck::new();
-        upstreams.set_health_check(health_check);
-        upstreams.health_check_frequency = Some(Duration::from_secs(1));
-
-        let health_check = background_service("health check", upstreams);
-        let health_checked_upstreams = health_check.task();
-        (health_check, health_checked_upstreams)
-    };
-    server.add_service(health_check);
+    let (service_a_hc, service_a_upstreams) =
+        build_lb_service::<RoundRobin>(&["1.1.1.1:443", "127.0.0.1:443"]);
+    let (service_b_hc, service_b_upstreams) =
+        build_lb_service::<RoundRobin>(&["1.0.0.1:443", "127.0.0.1:334"]);
+
+    server.add_service(service_a_hc);
+    server.add_service(service_b_hc);
```

And we should:

- Rename the `LBService` to something more meaningful, e.g. `GatewayService`
- Enable it to support multiple upstreams

```diff
- pub struct LBService {
-    balancer: Arc<LoadBalancer<RoundRobin>>,
-    name: String,
-}
+pub struct GatewayService {
+    service_a_upstreams: Arc<LoadBalancer<RoundRobin>>,
+    service_b_upstreams: Arc<LoadBalancer<RoundRobin>>,
+}
```

```diff
-        let upstream = self.balancer.select(b"", 256).unwrap();
+        let upstream = self.service_a_upstreams.select(b"", 256).unwrap();

        REQUESTS
            .with_label_values(&[&format!("{}", upstream.addr)])
            .inc();

-        let peer = Box::new(HttpPeer::new(upstream, true, self.name.clone()));
+        let peer = Box::new(HttpPeer::new(upstream, true, "one.one.one.one".to_string()));
```

```diff
-    let service = {
-        let service = LBService {
-            name: "one.one.one.one".to_string(),
-            balancer: upstreams,
-        };
-        let mut lb = http_proxy_service(&server.configuration, service);
-        lb.add_tcp("0.0.0.0:8001");
-        lb
-    };
-
-    server.add_service(lb);
+    let gateway = {
+        let service = GatewayService {
+            service_a_upstreams,
+            service_b_upstreams,
+        };
+        let mut gw = http_proxy_service(&server.configuration, service);
+        gw.add_tcp("0.0.0.0:8001");
+        gw
+    };
+
+    server.add_service(gateway);
```

The proxy should still work, however all requests should now only be going to `1.1.1.1:443`.

## 2. Add CTX to the request

This will be useful for selecting an upstream, and capturing metrics later.

Add a struct to capture the ctx data we need:

```rs
#[derive(Default)]
pub struct GWCtx {
    service_b: bool,
    upstream: String,
}
```

And make sure the Context is initiated, and filled via a `request_filter` call

```rs
impl ProxyHttp for GatewayService {
    type CTX = GWCtx;
    fn new_ctx(&self) -> Self::CTX {
        Self::CTX::default()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        ctx.upstream = session
            .req_header()
            .headers
            .get("X-Upstream-B")
            .map_or(false, |_| true);
        Ok(false)
    }
    // ..
```

## 3. Use CTX to choose an loadbalanced upstream

Pick the service load balancer by using the context:

```rs
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = if ctx.service_b {
            self.service_a_upstreams.select(b"", 256).unwrap()
        } else {
            self.service_b_upstreams.select(b"", 256).unwrap()
        };
        ctx.upstream = Some(upstream.addr.to_string());

        let peer = Box::new(HttpPeer::new(upstream, true, "one.one.one.one".to_string()));

        Ok(peer)
    }
```

## 4. Make sure the metrics are correct

We now can capture an upstream service, as well as the specific upstream.

```rs
lazy_static! {
    static ref REQUESTS: IntCounterVec = prometheus::register_int_counter_vec!(
        "requests_total",
        "Total number of requests.",
        &["service", "upstream"]
    )
    .unwrap();
}
```

And it makes more sense for us to now capture the metrics in a `response_filter` when the call comes back from the upstream.

```rs
    async fn response_filter(
        &self,
        _session: &mut Session,
        _upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let service = if ctx.service_b {
            "service_b"
        } else {
            "service_a"
        };
        let upstream = ctx.upstream.as_ref().unwrap();
        REQUESTS.with_label_values(&[service, upstream]).inc();
        Ok(())
    }
```

## 5. Test it

```sh
$ cargo run
```

Curl the load balanced gateway:

```sh
$ curl -H "X-Upstream-B: yes" http://localhost:8001/
$ curl http://localhost:8001/
$ curl http://localhost:8001/
```

And now see the metrics:

```sh
$ curl http://localhost:9090/
# HELP requests_total Total number of requests.
# TYPE requests_total counter
requests_total{service="service_a",upstream="1.0.0.1:443"} 2
requests_total{service="service_b",upstream="1.1.1.1:443"} 1
```
