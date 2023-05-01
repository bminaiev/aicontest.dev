#!/bin/bash
cd ..
killall example-client
RUST_LOG=info setsid cargo run --release --bin example-client -- --num-bots 20 --addr 188.166.195.142:7877 > prod/client.log 2>&1 &
tail -f prod/client.log

