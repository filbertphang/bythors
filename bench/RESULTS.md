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

## ivy (pldi18)

note that the ivy k-v store does not persist anything to disk:

> We note that etcd now displays worse performance than our Raft-based key-value store. We believe that this is primarily because etcd is writing to a ramdisk, while our key-value store doesn't have a persistence mechanism.

_(from pldi18 artifact, README.md, section "Differences with the submission\Etcd")_

so this is naturally going to be **significantly** faster, because it doesn't incur any storage access overhead.

Total time: 0.108757
Throughput: 7355.847071 reqs/s
404.000000 gets, avg = 0.000916
396.000000 puts, avg = 0.000939
