use async_trait::async_trait;
use lazy_static::lazy_static;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::lb::{
    health_check::TcpHealthCheck,
    selection::{BackendIter, BackendSelection, RoundRobin},
    LoadBalancer,
};
use pingora::proxy::{http_proxy_service, ProxyHttp, Session};
use pingora::server::{configuration::Opt, Server};
use pingora::services::background::{background_service, GenBackgroundService};
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
        &["service", "upstream"]
    )
    .unwrap();
}

#[derive(Default)]
pub struct GWCtx {
    service_b: bool,
    upstream: Option<String>,
}

pub struct GatewayService {
    service_a_upstreams: Arc<LoadBalancer<RoundRobin>>,
    service_b_upstreams: Arc<LoadBalancer<RoundRobin>>,
}

#[async_trait]
impl ProxyHttp for GatewayService {
    type CTX = GWCtx;
    fn new_ctx(&self) -> Self::CTX {
        Self::CTX::default()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        ctx.service_b = session
            .req_header()
            .headers
            .get("X-Upstream-B")
            .map_or(false, |_| true);
        Ok(false)
    }

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

        let peer = Box::new(HttpPeer::new(
            upstream,
            false,
            "one.one.one.one".to_string(),
        ));

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
}

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

fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    let mut server = Server::new(Some(opt)).unwrap();

    let (service_a_hc, service_a_upstreams) =
        build_lb_service::<RoundRobin>(&["127.0.0.1:8888", "127.0.0.1:443"]);
    let (service_b_hc, service_b_upstreams) =
        build_lb_service::<RoundRobin>(&["127.0.0.1:8889", "127.0.0.1:334"]);

    server.add_service(service_a_hc);
    server.add_service(service_b_hc);

    let gateway = {
        let service = GatewayService {
            service_a_upstreams,
            service_b_upstreams,
        };
        let mut gw = http_proxy_service(&server.configuration, service);
        gw.add_tcp("0.0.0.0:8001");
        gw
    };

    server.add_service(gateway);

    let prom_service = {
        let mut service = Service::prometheus_http_service();
        service.add_tcp("0.0.0.0:9090");
        service
    };
    server.add_service(prom_service);

    server.bootstrap();
    server.run_forever();
}
