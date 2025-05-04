# Benchmarking

All tests are done with: (on my device)

- 8 threads for sending data
- 100 requests per thread (so 800 requests total)
- 50 keys

| Key-Value Store   | Total Time (s) | Throughput (req/s) | GETs | Avg GET Time (s) | PUTs | Avg PUT Time (s) |
| ----------------- | -------------- | ------------------ | ---- | ---------------- | ---- | ---------------- |
| bythors           | 5.276967       | 151.602235         | 393  | 0.051792         | 407  | 0.052195         |
| etcd              | 2.168403       | 368.935126         | 409  | 0.021244         | 391  | 0.020934         |
| vard (verdi-raft) | 14.442923      | 55.390450          | 427  | 0.144262         | 373  | 0.143292         |
| ivy (pldi18)      | 0.108757       | 7355.847071        | 404  | 0.000916         | 396  | 0.000939         |

## Setup

**Requirements**: python 2.7 + urllib3 (for benchmarking script)

It is recommended to set up a virtual environment for benchmarking using `pyenv`:

```bash
pyenv install 2.7
pyenv virtualenv 2.7 bench-env
pyenv activate bench-env
pip install urllib3
```

Executables for etcd, vard, and ivy can be found in `bench/bin`.
Executable for bythors should be built using `cargo build -release --all-targets`.

Benchmarking scripts are provided in `bench/scripts`.
scripts should be run from the repo root directory, i.e., call the script like `bench/scripts/bench-xxx.sh`.

Persistent data for each server is stored in `data/`.

## Notes

The Ivy-Raft k-v store does not persist anything to disk:

> We note that etcd now displays worse performance than our Raft-based key-value store. We believe that this is primarily because etcd is writing to a ramdisk, while our key-value store doesn't have a persistence mechanism.

_(from pldi18 artifact, README.md, section "Differences with the submission\Etcd")_

So it is naturally going to be **significantly** faster, because it doesn't incur any storage access overhead.
