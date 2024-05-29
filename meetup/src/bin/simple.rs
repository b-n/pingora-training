use async_trait::async_trait;
use pingora::http::ResponseHeader;
use pingora::proxy::{http_proxy_service, ProxyHttp, Session};
use pingora::server::Server;
use pingora::upstreams::peer::HttpPeer;
use pingora::{Error, ErrorType, Result};

pub struct ResponseService;

#[async_trait]
impl ProxyHttp for ResponseService {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // We should never get here due to the request filter
        Err(Error::new(ErrorType::HTTPStatus(500)))
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        // Note: The below is bad practise, but is here just for a simple server that can respond
        // with 200s
        let body = "Hello!";

        let mut resp = ResponseHeader::build(200, Some(4)).unwrap();
        resp.insert_header("Server", "one.one.one.one").unwrap();
        resp.insert_header("Content-length", body.len()).unwrap();
        resp.insert_header("Cache-control", "private, no-store")
            .unwrap();

        let _ = session.write_response_header(Box::new(resp)).await;
        let _ = session.write_response_body(body.into()).await;
        Ok(true)
    }
}

fn main() {
    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let service = ResponseService {};
    let mut svc = http_proxy_service(&server.configuration, service);
    svc.add_tcp("0.0.0.0:8888");
    svc.add_tcp("0.0.0.0:8889");
    server.add_service(svc);

    server.run_forever();
}
