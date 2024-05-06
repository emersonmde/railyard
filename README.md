[![Rust Build](https://github.com/emersonmde/railyard/actions/workflows/rust.yml/badge.svg)](https://github.com/emersonmde/railyard/actions/workflows/rust.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Railyard

Railyard is an implementation of the Raft concensus algorithm using the Tonic
gRPC framework. This project aims to provide a reusable foundation for building
distributed systems that require high availability and fault tolerance.

> **⚠️ Warning: Experimental ⚠️**
>
> **Railyard** is currently in early development. It should not be used in
> production systems as it has not been tested and may still undergo
> significant changes and improvements.

## Getting Started

To incorporate Railyard into your project, add it as a dependency in your Cargo.toml:

```toml
[dependencies]
railyard = "0.1.0"
```

Initialize and run a Railyard cluster by including it in your Rust application:

```rust
use railyard::Railyard;

#[tokio::main]
async fn main() {
    let my_cluster = Railyard::new_server("node_id", vec!["peer1:port", "peer2:port"]).await;
    my_cluster.run().await;
}
```

For detailed examples and usage, see the [examples](examples) directory and refer to the
[API documentation](https://errorsignal.dev/railyard/railyard/index.html).

## Running Examples

### Cargo Run

To run the server example locally, set the port and at least 2 other peers:

```bash
cd examples/server
cargo run -- -p 8001 --peer 127.0.0.1:8002 --peer 127.0.0.1:8002 --peer 127.0.0.1:8003 &
cargo run -- -p 8002 --peer 127.0.0.1:8002 --peer 127.0.0.1:8001 --peer 127.0.0.1:8003 &
cargo run -- -p 8003 --peer 127.0.0.1:8002 --peer 127.0.0.1:8001 --peer 127.0.0.1:8002 &
```

To aid in spinning up a cluster, the `examples/server` directory contains a
`start_cluster.sh` script that will start a cluster with 3 nodes:

```bash
./examples/server/start_cluster.sh
```

### Docker Compose

To run the server using Docker Compose:

```bash
docker-compose up
```

This will startup a cluster with 3 nodes by default and automatically configure
the peers to listen on port 8000.

### Testing

Once the cluster is running the API can be tested with `grpcurl`:

```bash
grpcurl -plaintext -import-path ./proto -proto cluster_management.proto -d '{"entries": ["test"]}' '[::1]:8001' railyard.ClusterManagement/AppendEntries
```

# Resources

- [Raft Paper](https://raft.github.io/raft.pdf)
- [Tonic Framework](https://github.com/hyperium/tonic)

## License

This project is licensed under the [MIT license](LICENSE).
