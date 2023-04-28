#!/bin/bash
cd ..
killall game-server
nohup cargo run --release --bin game-server > prod/server.log 2>&1 &
tail -f prod/server.log
