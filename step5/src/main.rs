use async_trait::async_trait;
use lazy_static::lazy_static;
use pingora::http::RequestHeader;
use pingora::lb::{health_check::TcpHealthCheck, selection::RoundRobin, LoadBalancer};
use pingora::proxy::{http_proxy_service, ProxyHttp, Session};
use pingora::server::{configuration::Opt, Server};
use pingora::services::background::background_service;
use pingora::services::listening::Service;
use pingora::upstreams::peer::HttpPeer;
use pingora::Result;
use prometheus::{self, IntCounterVec};
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;

lazy_static! {
    static ref REQUESTS: IntCounterVec = prometheus::register_int_counter_vec!(
        "requests_total",
        "Total number of requests.",
        &["upstream"]
    )
    .unwrap();
}

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

        REQUESTS
            .with_label_values(&[&upstream.addr.to_string()])
            .inc();

        let peer = Box::new(HttpPeer::new(upstream, false, self.name.clone()));

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
    env_logger::init();

    let opt = Opt::from_args();
    let mut server = Server::new(Some(opt)).unwrap();

    let (upstreams, health_check) = {
        let mut upstreams: LoadBalancer<_> =
            LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443", "127.0.0.1:443"]).unwrap();

        let health_check = TcpHealthCheck::new();
        upstreams.set_health_check(health_check);
        upstreams.health_check_frequency = Some(Duration::from_secs(1));

        let health_check = background_service("health check", upstreams);
        let health_checked_upstreams = health_check.task();
        (health_checked_upstreams, health_check)
    };
    server.add_service(health_check);

    let service = {
        let service = LBService {
            name: "one.one.one.one".to_string(),
            balancer: upstreams,
        };
        let mut lb = http_proxy_service(&server.configuration, service);
        lb.add_tcp("0.0.0.0:8001");
        lb
    };

    server.add_service(service);

    let prom_service = {
        let mut service = Service::prometheus_http_service();
        service.add_tcp("0.0.0.0:9090");
        service
    };
    server.add_service(prom_service);

    server.bootstrap();
    server.run_forever();
}
