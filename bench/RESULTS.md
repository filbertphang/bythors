# introduction

all tests are done with: (on my device)

- 8 threads for sending data
- 100 requests (per thread)
- 50 keys

## bythors

Total time: 5.276967
Throughput: 151.602235 reqs/s
393.000000 gets, avg = 0.051792
407.000000 puts, avg = 0.052195

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
