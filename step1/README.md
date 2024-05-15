# Hello Pingora

1. Create the new project

```sh
cargo new lb
```

2. Add pingora dependency

```toml
pingora = { version = "0.2" }
```

3. Add boilerplate server code

```rs
use pingora::server::Server;

fn main() {
    let mut server = Server::new(None).unwrap();

    server.bootstrap();
    server.run_forever();
}
```

4. Run it `cargo run`

Congrats it should compile, and it'll run. But this server does absolutely nothing
