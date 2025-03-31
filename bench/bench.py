import time
import argparse
import random
import Queue
import threading
import multiprocessing
import ivy
import vard
import etcd

# from pldi18 artifact
# https://zenodo.org/records/2577103

start_time = None
DEBUG = False

def benchmark(client, requests, runtime, keys, put_percentage, thread_id, result_queue):
    def done():
        return (requests is not None and len(reqs) >= requests) or \
            (runtime is not None and time.time() - start_time >= runtime)

    gets = []
    puts = []
    reqs = []

    put_prob = put_percentage / 100.0
    i = 0
    try:
        while True:
            i += 1
            key = str(random.randint(0, keys))
            if random.random() < put_prob:
                start = time.time()
                # client.put('key' + key, str(i))
                client.put('key' + key + 'thread' + str(thread_id), str(i))
                end = time.time()
                puts.append(end-start)
                reqs.append((thread_id, end - start_time, end-start))
            else:
                start = time.time()
                # client.get('key' + key)
                client.get('key' + key + 'thread' + str(thread_id))
                end = time.time()
                gets.append(end-start)
                reqs.append((thread_id, end - start_time, end-start))
            if DEBUG:
                print 'Thread %d Done with %d requests' % (thread_id, i)
            if done():
                return
    except Exception as e:
        print "thread %d exiting due to unexpected exception " % thread_id, e
        raise e
    finally:
        result_queue.put((gets, puts, reqs))
        print "Thread %d exiting" % thread_id


def cluster(addrs):
    ret = []
    for addr in addrs.split(','):
        (host, _, port) = addr.partition(':')
        ret.append((host, int(port)))
    return ret

def main():
    global DEBUG
    global start_time
    parser = argparse.ArgumentParser()
    parser.add_argument('--service', default='ivy', choices=['etcd', 'vard', 'ivy'])
    parser.add_argument('--cluster', type=cluster, required=True)
    parser.add_argument('--requests', type=int)
    parser.add_argument('--time', type=float)
    parser.add_argument('--threads', default=50, type=int)
    parser.add_argument('--keys', default=100, type=int)
    parser.add_argument('--put-percentage', default=50, type=int)
    parser.add_argument('--debug', default=False, action='store_true')
    parser.add_argument('--tolerate-failover', action='store_true')
    parser.add_argument('--use-processes', action='store_true')
    args = parser.parse_args()
    print args

    if args.debug:
        DEBUG = True

    if args.requests is not None and args.time is not None:
        raise Exception("Only one of --requests and --time may be given")

    if args.requests is None and args.time is None:
        args.requests = 1000

    Q = Queue.Queue
    T = threading.Thread

    if args.use_processes:
        Q = multiprocessing.Queue
        T = multiprocessing.Process

    Client = ivy.Client
    if args.tolerate_failover:
        Client = vard.FailoverTolerantClient
    if args.service == 'vard':
        Client = vard.Client
    if args.service == 'etcd':
        Client = etcd.Client

    gets = []
    puts = []
    reqs = []

    result_queue = Q()

    if not args.tolerate_failover:
        host, port = Client.find_leader(args.cluster)
    else:
        # block until leader election
        Client(args.cluster)
    threads = []
    start_time = time.time()
    print "start time: %s" % start_time
    for thread_id in range(args.threads):
        print 'Starting thread %s' % thread_id
        if not args.tolerate_failover:
            c = Client(host, port)
        else:
            c = Client(args.cluster)
        thr = T(target=benchmark, args=(c, args.requests, args.time, args.keys, args.put_percentage, thread_id, result_queue))
        threads.append(thr)
        thr.start()
        # time.sleep(10)

    for thr in threads:
        (g, p, r) = result_queue.get()
        gets.extend(g)
        puts.extend(p)
        reqs.extend(r)

    for thr in threads:
        thr.join()
    end = time.time()

    # print 'Requests:'
    # for (tid, ts, latency) in reqs:
    #     print 'REQUEST: THREAD %s TIME %s LATENCY %s' % (tid, ts, latency)
    print 'Total time: %f' % (end - start_time)
    print 'Throughput: %f reqs/s' % ((len(gets) + len(puts)) / (end - start_time))
    print '%f gets, avg = %f' % (len(gets), sum(gets)/len(gets))
    print '%f puts, avg = %f' % (len(puts), sum(puts)/len(puts))

if __name__ == '__main__':
    main()
