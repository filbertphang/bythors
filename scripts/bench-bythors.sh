#!/usr/bin/env bash

function start-bythors {
  NODE=${1}
  target/release/bythors ${NODE} 3 &
  sleep 1
}

echo "=== bench-bythors ==="
echo "starting instances"

start-bythors 1
start-bythors 2
start-bythors 3

# sleep so that initial election can resolve
echo "waiting for election"
sleep 20

echo "starting benchmarks"

python2 bench/bench.py --service bythors --keys 50 --cluster "localhost:8001,localhost:8002,localhost:8003" --threads 8 --requests 100

killall -9 bythors > /dev/null

echo "done!"