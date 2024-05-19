# CLI Options and Configuration

Until all config is compiled into our code. Let's extract some.

## 1. Get logging working

This isn't entirely necessary, but helps to diagnose what's going on.

Add `env_logger` dependency:

```toml
env_logger = { version = "0.11.3" }
```

Ensure `env_logger` is initialized in `main()`

```rs
// ..
fn main() {
    env_logger::init();
// ..
```

## 2. Enable the server to accept CLI Options 

Add `structopt` dependency:

```toml
structopt = { version = "0.3" }
```

Import some new dependencies:

```rs
use pingora::server::{configuration::Opt, Server};
use structopt::StructOpt; 
```

Update the server to use the configuration options:

```rs
// ..
fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();
    // ..
```

## 3. Configure the config

Create a file `conf.yaml`

```yaml
---
version: 1
threads: 2
pid_file: /tmp/pingora.pid
error_log: /tmp/pingora_err.log
upgrade_sock: /tmp/pingora.sock
```

## 4. Run the process daemonized

```sh
RUST_LOG=INFO cargo run -- -d -c conf.yaml
```

`/tmp/pingora.pid` should have the process pid for the running pingora server now.
