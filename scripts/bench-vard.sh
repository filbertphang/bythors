#!/usr/bin/env bash

# N=2
# PORT=800${N}
# bench/bin/vard.native -dbpath "/tmp/vard-$PORT" \
#               -port "$PORT" \
#               -node 0,localhost:9000 -node 1,localhost:9001 -node 2,localhost:9002 \
#               -me "${N}"

function start-vard {
  PORT=800${1}
  bench/bin/vard.native -dbpath "/tmp/vard-$PORT" \
                -port "$PORT" \
                -node 0,localhost:9000 -node 1,localhost:9001 -node 2,localhost:9002 \
                -me "$1" \
                > "/tmp/vard-${PORT}.log" &
  sleep 1
}

echo "=== bench-vard ==="
echo "starting instances"

start-vard 0
start-vard 1
start-vard 2

# sleep so that initial election can resolve
echo "waiting for election"
sleep 5

echo "starting benchmarks"

python2 bench/bench.py --service vard --keys 50 --cluster "localhost:8000,localhost:8001,localhost:8002" --threads 8 --requests 100

killall -9 vard.native > /dev/null

echo "done!"