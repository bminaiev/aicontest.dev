#!/bin/bash
cd ..
RUST_LOG=debug cargo run --bin example-client -- --num-bots 5 --address 188.166.195.142:7877