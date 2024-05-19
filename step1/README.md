# Hello Pingora

The below assumes you already have a working rust toolchain (e.g. you can run `cargo`).

## 1. Create a new project

```sh
cargo new pingora
cd pingora
```

## 2. Add pingora dependency to Cargo.toml

`Cargo.toml`:

```toml
[dependencies]
pingora = { version = "0.2" }
```

## 3. Add boilerplate server code

`src/main.rs`:

```rs
use pingora::server::Server;

fn main() {
    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    server.run_forever();
}
```

## 4. Run it!

```sh
cargo run
```

Congrats it should compile, and it should run. This server does absolutely nothing though, so it's not helpful [yet!].
