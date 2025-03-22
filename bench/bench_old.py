import time
import argparse
import random
import subprocess

gets = []
puts = []
reqs = []
DEBUG = False

# python3 translation + modification of verdi-raft/bench/bench.py

def benchmark(requests, keys, put_percentage, n):
    print("in bench")
    p : subprocess.Popen = subprocess.Popen(
        ["target/release/examples/store"],
        stdin = subprocess.PIPE,
        stdout = subprocess.PIPE,
        stderr = subprocess.STDOUT,
    )
    print("after popening")
    random.seed(n)
    put_prob = put_percentage / 100.0
    i = 0

    while True:
        i += 1
        key = str(random.randint(0, keys))
        if random.random() < put_prob:
            q = f'put {key} {str(i)}\n'.encode()
            start = time.time()
            p.stdin.write(q)
            p.stdin.flush()
            p.stdout.readline()
            end = time.time()
            puts.append(end - start)
            reqs.append((n, end, end - start))
        else:
            q = f'get {key}\n'.encode()
            start = time.time()
            p.stdin.write(q)
            p.stdin.flush()
            p.stdout.readline()
            end = time.time()
            gets.append(end - start)
            reqs.append((n, end, end - start))
        if DEBUG:
            print(f'Process {n} Done with {i} requests')
        if len(reqs) >= requests:
            p.terminate()
            return

def main():
    global DEBUG
    parser = argparse.ArgumentParser()
    parser.add_argument('--requests', default=1000, type=int)
    # parser.add_argument('--threads', default=50, type=int)
    parser.add_argument('--iterations', default=1, type=int)
    parser.add_argument('--keys', default=100, type=int)
    parser.add_argument('--put-percentage', default=50, type=int)
    parser.add_argument('--debug', default=False, action='store_true')
    args = parser.parse_args()

    if args.debug:
        print("debug true")
        DEBUG = True

    start = time.time()
    for i in range(args.iterations):
        print('Starting iteration {}'.format(i))
        benchmark(args.requests, args.keys, args.put_percentage, i)
        # thr = t.Thread(target=benchmark, args=(args.requests, args.keys, args.put_percentage, i))
        # threads.append(thr)
        # thr.start()
        # time.sleep(10)
    # for thr in threads:
    #     thr.join()
    end = time.time()

    print('Requests:')
    for tid, ts, latency in reqs:
        print('REQUEST: THREAD {} TIME {} LATENCY {}'.format(tid, ts, latency))
    print('Total time: {}'.format(end - start))
    print('{} gets, avg = {}'.format(len(gets), sum(gets) / len(gets) if gets else 0))
    print('{} puts, avg = {}'.format(len(puts), sum(puts) / len(puts) if puts else 0))
    print('Requests:')

if __name__ == '__main__':
    main()
