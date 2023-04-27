#!/bin/bash
cd ..
RUST_LOG=info cargo run --release --bin example-client -- --num-bots 50 --addr 188.166.195.142:7877