# Performance measurements

For current, controlled 100-node measurements, including the SGD stress method,
see [the 100-node performance report](performance-100.md). The larger-size and
rendered-document measurements below predate the latest stress and spring
optimizations; their hashes are preserved with the raw results.

The original measurements below used the package name `graph-layout`, before
the rename to `fast-layout`. Their recorded hashes remain unchanged. See
[the diagraph comparison](diagraph-comparison.md) for measurements of the renamed build.

Measured on 2026-09-12 on an Intel Xeon Gold 6226R at 2.90 GHz, Linux x86_64,
Rust 1.93.1, Julia 1.12.6, and Typst 0.15.1. This is a desktop machine with other
applications running, not an isolated benchmark host. Times are medians of three
runs; the JSON records retain minimum and maximum native times or individual
Typst samples. Native runs have one warmup. Julia and OpenBLAS use one thread.

The native graph has n nodes and edges at circulant offsets 1, 7, and 31.
Initial coordinates are `(sin(i), cos(i))`, with `sin(2i)` added in 3D, for
one-based node i. Stress and spring use zero tolerance and the exact update
counts listed below. Julia consumes its iterator directly to avoid the callable
off-by-one bug, as documented in [the baseline](baseline-julia.md).

## Native computation

| Method, nodes=1000 | Dimensions | Updates | Rust median | Julia median |
| --- | ---: | ---: | ---: | ---: |
| Exact stress | 2 | 5 | 141 ms | 4877 ms |
| Exact stress | 3 | 5 | 160 ms | 4807 ms |
| Exact spring | 2 | 20 | 179 ms | 264 ms |
| Exact spring | 3 | 20 | 214 ms | 282 ms |
| Spring, theta=0.7 | 2 | 20 | 46 ms | Different approximation |
| Spring, theta=0.7 | 3 | 20 | 93 ms | Different approximation |
| Sparse spectral, residual target 1e-8 | 2 | 8 outer iterations | 23 ms | Not measured |

At 10,000 nodes, accelerated spring takes 709 ms in 2D and 2082 ms in 3D for
20 updates. Dense stress is not intended for that size. The earlier full dense
spectral solver took about 964 ms at 1000 nodes; partial iteration removed that
bottleneck on this graph.

The target of beating Julia on every measured size was **not met**. For example,
100-node exact spring takes roughly 4–5 ms here, versus Julia's 2.93 ms in 2D.
The improvements at 1000 nodes do not imply a universal speedup.

Raw results are in `benchmarks/native-stress*.json`,
`benchmarks/native-spring-exact*.json`, `benchmarks/native-spring-accelerated*.json`,
and `benchmarks/native-spectral.json`. Each file contains both `compute` and
`cbor` measurements. CBOR timings include request encoding, decoding inside the
engine, computation, response encoding, and response decoding. Small differences
between these medians can be smaller than measurement noise.

## Approximation quality

Barnes–Hut force tests compare the same point cloud with the exact kernel in
2D and 3D. At theta=0.7 the aggregate relative force error is below 5%; theta=0.3
improves it, and theta=0 matches exact forces to numerical tolerance. Tests also
cover coincident nodes and self-force exclusion.

Force error does not bound the final layout error after repeated updates.
`benchmarks/native-spring-quality-2d.json` and `-3d.json` compute the exact
Fruchterman–Reingold energy of each final layout outside the timed region. The
energy sums `-k² log(distance)` over pairs and `distance³ / (3k)` over edges.
Lower is better; energies can be negative, so ratios are not meaningful.

| 1000 nodes, 20 updates | Exact energy | theta=0.7 energy |
| --- | ---: | ---: |
| 2D | 98.95 | 1435.04 |
| 3D | -888.95 | -988.12 |

The short 2D run shows a substantial quality cost on this symmetric input.
Approximation can follow a different trajectory and can also find a lower
energy, as in the 3D case. Use `theta: 0` when exact forces matter. These fixed
iteration timings do not claim convergence or equal final quality.

At the default 100-update limit on the same 1000-node 2D input, exact spring
takes 862 ms with energy -7494.13. Theta=0.3 takes 594 ms with energy -7435.52,
and theta=0.7 takes 217 ms with energy -6876.92. At 200 updates both approximate
runs reach lower energy than the exact run, so energy differences depend on the
trajectory and stopping point. Raw records are in `benchmarks/native-spring-long-*.json`.

The automatic setting keeps theta=0.7 for speed; choose theta=0.3 for a closer
approximation or zero for exact forces. At 256 nodes, exact 2D/3D runs take
12.1/14.6 ms, versus 7.2/12.3 ms with theta=0.7, supporting the automatic
crossover there. At 100 nodes approximation is slower, so the default is exact.

Shell and Buchheim take about 2.04 and 1.28 ms respectively at 10,000 nodes.
`benchmarks/native-geometric.json` also records 10, 100, and 1000 nodes.

## Typst compilation

Fresh-process measurements are in `benchmarks/typst.json`, including the bundled
WASM SHA-256. Inputs are fixed connected random graphs of about 2n edges,
shared with the diagraph-layout controls. They differ from the native circulant
inputs above. Each case has three fresh processes and a 20-second per-run limit.

| Fresh compile case | Median |
| --- | ---: |
| baseline | 0.258 s |
| stress-100 | 0.219 s |
| spring-100 | 0.184 s |
| spectral-100 | 0.096 s |
| diagraph-neato-100 | 1.331 s |
| diagraph-sfdp-100 | 0.568 s |
| stress-500 | 4.856 s |
| spring-500 | 1.848 s |
| spectral-500 | 0.885 s |
| diagraph-neato-500 | 20 s timeout |
| diagraph-sfdp-500 | 3.694 s |
| stress-1000 | 20 s timeout |
| spring-1000 | 4.153 s |
| spectral-1000 | 2.633 s |
| diagraph-neato-1000 | 20 s timeout |
| diagraph-sfdp-1000 | 9.430 s |
| shell-10000 | 0.053 s |
| buchheim-10000 | 0.084 s |

These timings include Typst startup, document evaluation, WASM loading, CBOR,
layout, and output compilation. The plugin returns coordinates; Graphviz also
performs drawing and edge handling. Graphviz's algorithms and stopping rules
differ. The comparison measures document workloads, not the programming
language alone. The plain-document baseline can exceed a simple coordinate-only
document because their typesetting work differs; subtracting it is not a reliable
kernel estimate.

Exact stress at 1000 nodes exceeded the 20-second limit in the tested Typst
workload. Native speed does not imply interactive speed inside Typst's WASM
runtime. For large figures, precomputing with the native library avoids repeated
WASM work; smaller graphs can use the bundled plugin directly.

`benchmarks/typst-incremental.json` separates the first compilation, an edit to
unrelated text, and an edit to the layout seed in a running `typst watch` process.
The script records both elapsed wall time and Typst trace compilation time.
Wall time includes file-watcher debounce. Text-only edits can reuse the cached
layout, whereas changing the seed recomputes it. This measures observed cache
behavior for these documents and Typst version, not a cache guarantee.

For spring at 500 nodes, median trace times are 1834 ms for first compilation,
5.71 ms for a text-only edit, and 1840 ms for a seed change. Corresponding wall
times are 2090, 108, and 1943 ms.

## Memory and artifact size

GNU `time` measured peak native process RSS of 26,896 KiB for 1000-node 3D
stress with five updates, and 5,340 KiB for 4096-node 3D sparse spectral with a
1e-8 residual target. Each measurement runs the benchmark executable with one
warmup and one measured call for both native and CBOR paths. These are process
peaks, including allocator reuse, rather than per-call allocations. Commands
use the same circulant input as the native timing table. Raw results are in
`benchmarks/memory-*.txt`; they do not measure Typst's process memory.

The original bundled WASM was 254,162 bytes. Its SHA-256 and release settings are recorded
in `benchmarks/build.json`. A rebuild into a clean target directory produced
identical bytes on this platform. The build uses path remapping; cross-platform
reproducibility has not been independently measured.

## Reproduction

```sh
cargo bench -p fast-layout-engine --bench layouts -- \
  --sizes 100,500,1000 --algorithms stress --dim 2 \
  --iterations 5 --tolerance 0 --repeats 3 --format json
cargo bench -p fast-layout-engine --bench layouts -- \
  --sizes 100,500,1000 --algorithms spring --dim 2 \
  --iterations 20 --theta 0 --tolerance 0 --repeats 3 --format json
cargo bench -p fast-layout-engine --bench layouts -- \
  --sizes 100,500,1000 --algorithms spectral --dim 2 \
  --iterations 100 --tolerance 1e-8 --repeats 3 --format json
make bench-typst BENCH_TYPST_ARGS='--compare --repeats 3 --timeout 20'
make bench-incremental
```

Cargo runs the benchmark from the crate directory. Use an absolute `--output`
path when saving JSON. The Typst scripts accept `--help` for bounded size,
algorithm, repeat, and timeout controls.

## Coverage and limits

Julia fixtures verify fixed exact spring coordinates to 2e-10 and stress
geometry/objectives to 2e-8. Spectral tests check generalized eigenvalues and
residuals, including repeated eigenvalues and 5D embeddings. Buchheim fixtures
include 24 varied trees. The release-only jagmesh case covers 936 nodes in 2D
and 3D. Typst tests check numerical coordinates through the bundled WASM.

This release does not include a full timing matrix for every tested topology,
a per-stage stress profile, or an exhaustive search for the best Barnes–Hut
crossover on every graph family. Those are additional measurements, not grounds
for claiming all graph layouts are "ultra fast". Memory and dense-method limits
remain relevant even with faster arithmetic.
