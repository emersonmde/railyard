[![Rust Build](https://github.com/emersonmde/railyard/actions/workflows/rust.yml/badge.svg)](https://github.com/emersonmde/railyard/actions/workflows/rust.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Railyard

Railyard is an implementation of the Raft concensus algorithm using Tonic

## Running

### Cargo Run

To run the server locally, set the port and at least 2 other peers:

```bash
cd examples/server
cargo run -- -p 8001 --peer 127.0.0.1:8002 --peer 127.0.0.1:8003
```

```text
Usage: railyard --port <PORT> --peer <PEER_ADDRESS>

Options:
  -p, --port <PORT>          Port used for management API
      --peer <PEER_ADDRESS>  The address of a peer node
  -h, --help                 Print help
  -V, --version              Print version
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

- [Raft](https://raft.github.io/raft.pdf)
- [Tonic](https://github.com/hyperium/tonic)

## License

This project is licensed under the [MIT license](LICENSE).
