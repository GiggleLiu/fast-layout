# Native SGD benchmark at 100 nodes

This historical benchmark compares fast-layout's native Rust stress SGD with
the authors' C++ [`s_gd2`](https://github.com/jxz12/s_gd2) implementation. It
was recorded on September 12, 2026 with an earlier fast-layout build and has not
been rerun for the current package. Treat it as a record of that build, not a
claim about current Typst execution time.

Both implementations used the same 100-node graphs, edges, initial coordinates,
seed, 15 epochs, and epsilon 0.1. Both optimize dense all-pairs stress. The C++
timings include the Python wrapper; the Rust timings measure direct `compute`
calls. Each result is the median of five serial samples after one untimed
warm-up, on an Intel Xeon Gold 6226R.

| Graph | Edges | Rust | C++ `s_gd2` | Rust objective difference |
| --- | ---: | ---: | ---: | ---: |
| Path | 99 | 2.404 ms | 1.579 ms | +10.72% |
| Cycle | 100 | 2.347 ms | 1.599 ms | +0.01% |
| 10x10 grid | 180 | 3.083 ms | 1.667 ms | -0.005% |
| Connected random | 194 | 3.000 ms | 1.800 ms | +0.75% |

The objective difference is `(Rust / C++ - 1)` for the same exact stress sum.
A negative value means Rust produced slightly lower stress. Equal seeds do not
produce the same pair order because the implementations use different random
number generators. The 15-epoch, epsilon 0.1 schedule comes from Zheng, Pawar,
and Goodman's [SGD graph drawing paper](https://arxiv.org/abs/1710.04626).

The raw records preserve the exact graph data, coordinates, objectives, package
versions, source revision, build hashes, and individual samples:

- [`references-100.json`](benchmarks/references-100.json) records `s_gd2` 1.8.1.
- [`rust-reference-100.json`](benchmarks/rust-reference-100.json) records the
  matched Rust run.

To reproduce the comparison from the current source tree:

```sh
make pkgroot plugin
python3 -m venv /tmp/fast-layout-reference-env
/tmp/fast-layout-reference-env/bin/pip install s_gd2==1.8.1 python-igraph==1.0.0 igraph==1.0.0
/tmp/fast-layout-reference-env/bin/python scripts/bench_references.py
```

The reproduction creates a new measurement for the checked-out source. It will
not recreate the historical Rust result unless that earlier source revision is
checked out.
