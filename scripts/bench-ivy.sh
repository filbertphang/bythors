#!/usr/bin/env bash

function start-ivy {
  PORT=800${1}
  bench/bin/ivy-server --log --node-id ${1} \
                           --client-port ${PORT} \
                           --cluster "localhost:8000,localhost:8001,localhost:8002" &
  sleep 1
}

echo "=== bench-ivy ==="
echo "starting instances"

start-ivy 0
start-ivy 1
start-ivy 2

# sleep so that initial election can resolve
echo "waiting for election"
sleep 5

echo "starting benchmarks"

python2 bench/bench.py --service ivy --keys 50 --cluster "localhost:8000,localhost:8001,localhost:8002" --threads 8 --requests 100

killall -9 ivy-server > /dev/null

echo "done!"