# 100-node performance

This report measures the current 285,605-byte WASM plugin on a controlled
100-node connected random graph with 194 edges. It separates the first plugin
call in a fresh Typst process from later calls that reuse the loaded module but
still compute a new layout. Lower is faster.

## Typst plugin calls

| Method | Older build, first | Older build, warm uncached | Current, first | Current, warm uncached |
| --- | ---: | ---: | ---: | ---: |
| Shell | 4.200 ms | 0.558 ms | 4.306 ms | 0.345 ms |
| Spectral | 75.093 ms | 67.798 ms | 75.150 ms | 67.446 ms |
| Stress majorization, 100 updates | 200.455 ms | 192.162 ms | 143.699 ms | 140.813 ms |
| Spring, 100 updates | 159.299 ms | 156.932 ms | 54.089 ms | 46.723 ms |
| Stress SGD, 5 epochs | n/a | n/a | 16.582 ms | 10.106 ms |
| Stress, default SGD with 15 epochs | n/a | n/a | 25.987 ms | 21.048 ms |

These are medians from Typst timing events, not differences between whole-process
times. Importing the package alone spent a median 6.21 ms in Typst's `load
plugin` event in the current run. The profiler does not subtract that or the
plain-document baseline. Typst 0.15.1 reports plugin exports only as `call
plugin`, so the controlled document identifies which export each span measures.

Each first-call case ran in a new Typst process. Each repeated-call case made
five calls in one document with distinct seeds to defeat memoization. The warm
columns use calls two through five and exclude each document's first call. All
documents produced the same blank one-page output and asserted a 100-position
result. Three serial repetitions were run on an Intel Xeon Gold 6226R. The saved
traces contain all samples.

SGD replaces factorization and repeated global solves with shuffled pair updates.
The 2D spring kernel now computes each force directly with fixed-size coordinates;
its force model and 100-update limit are unchanged.

The warm Typst default SGD15 result is 21 ms, while matched native Rust computation is
roughly 2 to 3 ms. Typst 0.15.1 executes plugins through the
[`wasmi` interpreter](https://github.com/typst/typst/blob/v0.15.1/crates/typst-library/src/foundations/plugin.rs),
and the plugin call also crosses the CBOR boundary. Native speed therefore does
not imply single-digit millisecond execution inside Typst.

## Matched native SGD comparison (historical)

This native comparison was recorded with the previous kernel build and was not
rerun for the current plugin profile. Its algorithms and benchmark configuration
are unchanged. It uses the same 100 nodes, edges, initial coordinates, seed, 15
epochs, and epsilon 0.1 for Rust and the authors' C++
[`s_gd2`](https://github.com/jxz12/s_gd2) implementation. Both optimize dense
all-pairs stress. The Python wrapper is included in the C++ timing; Rust timings
below are direct `compute` calls.

| Graph | Edges | Rust | C++ `s_gd2` | Rust objective difference |
| --- | ---: | ---: | ---: | ---: |
| Path | 99 | 2.404 ms | 1.579 ms | +10.72% |
| Cycle | 100 | 2.347 ms | 1.599 ms | +0.01% |
| 10x10 grid | 180 | 3.083 ms | 1.667 ms | -0.005% |
| Connected random | 194 | 3.000 ms | 1.800 ms | +0.75% |

The objective difference is `(Rust / C++ - 1)` for the same exact stress sum;
negative means the Rust result had slightly lower stress. The path objectives
are 0.015182 for Rust and 0.013712 for C++, summed over all 4,950 unordered
pairs without rescaling the coordinates. Equal seed values do not imply the
same pair order because the implementations use different random generators. SGD15 is the measured
speed-quality setting from Zheng, Pawar, and Goodman's
[SGD graph drawing paper](https://arxiv.org/abs/1710.04626). It uses the same
dense stress objective as majorization, but it is a different
optimizer with a different update schedule and convergence behavior. The table
does not compare five-epoch quality.

The package defaults to SGD with 15 updates. Arbitrary dimensions, initial
positions, and pins remain available. `stress-method: "majorization"` selects
majorization with an automatic 100-update budget. An explicit `iterations`
value overrides either budget. Unpinned 2D SGD requests use its specialized hot
path, while other dimensions and pinned requests use the general implementation.

For context, we inspected the official
[OGDF `StressMinimization` documentation](https://ogdf.github.io/doc/ogdf/classogdf_1_1_stress_minimization.html)
and [source](https://github.com/ogdf/ogdf/blob/master/src/ogdf/energybased/StressMinimization.cpp).
It initializes with 50-pivot MDS when no layout is supplied, then updates each
node from weighted votes of the other nodes without a Cholesky solve. We did not build a
matched OGDF benchmark and copied no GPL code, so no OGDF timing appears here.
The results above are workload-specific and do not establish a universally
fastest layout implementation.

## Reproduction and records

Run the Typst matrix with:

```sh
python3 scripts/profile_typst.py --repeats 3 --repeated-calls 5
```

The [current](benchmarks/profile-100-current.json),
[before](benchmarks/profile-100-before.json), and
[after](benchmarks/profile-100-after.json) records contain raw wall times,
trace spans, compiler and machine metadata, source hashes, and WASM hashes. The
before and after reports are historical. The current plugin SHA-256 is
`55c735231dfdf15f3b45089150f318035459373bb4e8e5d0928c5b33c549a847`.

The native reference scripts used an isolated temporary Python environment with
`s_gd2` 1.8.1, `python-igraph` 1.0.0, and `igraph` 1.0.0. Their five-sample raw
records, package versions, source revision, objectives, and coordinates are in
[`references-100.json`](benchmarks/references-100.json) and
[`rust-reference-100.json`](benchmarks/rust-reference-100.json).

```sh
make pkgroot plugin
python3 -m venv /tmp/fast-layout-reference-env
/tmp/fast-layout-reference-env/bin/pip install s_gd2==1.8.1 python-igraph==1.0.0 igraph==1.0.0
/tmp/fast-layout-reference-env/bin/python scripts/bench_references.py
```
