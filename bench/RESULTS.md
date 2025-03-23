# introduction

all tests are done with: (on my device)

- 8 threads
- 100 requests (per thread)
- 50 keys

## bythors

(8 threads, broken pipes)
Total time: 10.103248
Throughput: 1.484671 reqs/s
7.000000 gets, avg = 0.025840
8.000000 puts, avg = 0.024830

(1 thread, no broken pipes)
Total time: 5.004795
Throughput: 19.980838 reqs/s
47.000000 gets, avg = 0.049317
53.000000 puts, avg = 0.050615

## etcd

Total time: 2.168403
Throughput: 368.935126 reqs/s
409.000000 gets, avg = 0.021244
391.000000 puts, avg = 0.020934

## vard (verdi-raft)

Total time: 14.442923
Throughput: 55.390450 reqs/s
427.000000 gets, avg = 0.144262
373.000000 puts, avg = 0.143292
