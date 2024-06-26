#!/bin/bash

trap "kill 0" EXIT

export RUST_LOG=debug

cargo run -- -p 8001 --peer http://127.0.0.1:8002 --peer http://127.0.0.1:8003 --peer http://127.0.0.1:8004 &
cargo run -- -p 8002 --peer http://127.0.0.1:8001 --peer http://127.0.0.1:8003 --peer http://127.0.0.1:8004 &
cargo run -- -p 8003 --peer http://127.0.0.1:8001 --peer http://127.0.0.1:8002 --peer http://127.0.0.1:8004 &
cargo run -- -p 8004 --peer http://127.0.0.1:8001 --peer http://127.0.0.1:8002 --peer http://127.0.0.1:8003 &

wait
