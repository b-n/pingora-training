use async_trait::async_trait;
use pingora::lb::{selection::RoundRobin, LoadBalancer};
use pingora::proxy::{http_proxy_service, ProxyHttp, Session};
use pingora::server::Server;
use pingora::upstreams::peer::HttpPeer;
use pingora::Result;

pub struct LBService {
    balancer: LoadBalancer<RoundRobin>,
    name: String,
}

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

fn main() {
    let mut server = Server::new(None).unwrap();

    let service = {
        let upstreams = LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).unwrap();
        let service = LBService {
            name: "one.one.one.one".to_string(),
            balancer: upstreams,
        };
        let mut lb = http_proxy_service(&server.configuration, service);
        lb.add_tcp("0.0.0.0:8001");
        lb
    };

    server.add_service(service);

    server.bootstrap();
    server.run_forever();
}
