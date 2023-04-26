#!/bin/bash
cd ../web
nohup SERVER_URL=ws://188.166.195.142:7878 trunk serve --release --address 0.0.0.0 --ignore . &