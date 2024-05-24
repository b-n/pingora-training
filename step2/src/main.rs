use async_trait::async_trait;
use pingora::http::RequestHeader;
use pingora::lb::{selection::RoundRobin, LoadBalancer};
use pingora::proxy::{http_proxy_service, ProxyHttp, Session};
use pingora::server::Server;
use pingora::upstreams::peer::HttpPeer;
use pingora::Result;
use std::sync::Arc;

pub struct LBService {
    balancer: Arc<LoadBalancer<RoundRobin>>,
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

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request
            .insert_header("Host", "one.one.one.one")
            .unwrap();
        Ok(())
    }
}

fn main() {
    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let upstreams = {
        let upstreams = LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).unwrap();
        Arc::new(upstreams)
    };

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
