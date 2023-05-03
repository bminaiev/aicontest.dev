#!/bin/bash
cd ../web
killall trunk
SERVER_URL=wss://aicontest.dev:7879/ setsid trunk serve --release --address 0.0.0.0 --ignore . > ../prod/web.log 2>&1  &
tail -f ../prod/web.log
