
[![Rust](https://github.com/emersonmde/railyard/actions/workflows/rust.yml/badge.svg)](https://github.com/emersonmde/railyard/actions/workflows/rust.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Railyard

An event streaming platform (WIP)

# Running

To run the server locally, set the port and at least 2 other peers:
```bash
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


# Resources
- [Raft](https://raft.github.io/raft.pdf)
- [Tonic](https://github.com/hyperium/tonic)

## License

This project is licensed under the [MIT license](LICENSE).
