# Health checks

> But what if one of those upstreams are down?

## 1. Add a broken server

Add `127.0.0.1:443` to the list of upstreams:

```rs
fn main() {
    // ...
    let service = {
        let upstreams =
            LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443", "127.0.0.1:443"]).unwrap();
    // ...
}
```

And now curl it a 3 times:

```sh
curl --verbose http://localhost:8001
```

You should see two of the requests return a 200, and the last one a 502

## 2. Add a TCP Healthcheck

Add the new dependencies:

```rs
use pingora::lb::health_check::TcpHealthCheck;
use pingora::services::background::background_service;
use std::time::Duration;
```

And change the upstreams as follows:

```rs
// ..
fn main() {
    // ..
    let upstreams = {
        let mut upstreams: LoadBalancer<_> =
            LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443", "127.0.0.1:443"]).unwrap();

        let health_check = TcpHealthCheck::new();
        upstreams.set_health_check(health_check);
        upstreams.health_check_frequency = Some(Duration::from_secs(1));

        let health_check_service = background_service("health check", upstreams);
        let health_checked_upstreams = health_check_service.task();
        health_checked_upstreams
    };
```

Now run curl at least 3 times.

Our broken server is still being called.

## 3. Add the `health_check_service` to the server

The `health_check_service` is declared, but never used.

```rs
    // ..
    let (upstreams, health_check) = {
        // ..
        
        let health_check = background_service("health check", upstreams);
        (health_checked_upstreams, health_check)
    };
    server.add_service(health_check);
    
    let service = {
        // ..

    };
    // ..
```

Now call it again with curl, and every response should be valid.
