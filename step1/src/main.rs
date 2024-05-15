use pingora::server::Server;

fn main() {
    let mut server = Server::new(None).unwrap();

    server.bootstrap();
    server.run_forever();
}
