#!/bin/bash
cd ../web
killall trunk
SERVER_URL=ws://188.166.195.142:7878 nohup trunk serve --release --address 0.0.0.0 --ignore . > ../prod/web.log 2>&1  &
