use async_trait::async_trait;
use pingora::lb::{selection::RoundRobin, LoadBalancer};
use pingora::server::Server;
use pingora::Result;

fn main() {
    let mut server = Server::new(None).unwrap();

    let mut upstreams: LoadBalancer<RoundRobin> =
        LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443", "127.0.0.1:443"]).unwrap();

    server.add_service(upstreams);

    server.bootstrap();
    server.run_forever();
}
