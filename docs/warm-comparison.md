# Warm layout comparison

Measured on 2026-09-13 with Typst 0.15.1 on an Intel Xeon Gold 6226R, Linux x86_64.
Both packages ran on the same machine, serially, with cases interleaved over
three repetitions. The graph has 100 nodes and 194 undirected edges, generated
by `scripts/bench_typst.py::graph(100)`. Every case receives the same edge list;
shell uses the node count to place points on a circle.

| Package / layout | Warm API | Warm WASM export |
| --- | ---: | ---: |
| fast-layout 0.1.0: Stress SGD, default 15 passes | 21.437 ms | 21.355 ms |
| fast-layout 0.1.0: Stress majorization, 100 updates | 140.452 ms | 140.339 ms |
| fast-layout 0.1.0: Spring, 100 updates | 46.997 ms | 46.912 ms |
| fast-layout 0.1.0: Spectral | 67.763 ms | 67.689 ms |
| fast-layout 0.1.0: Shell | 0.749 ms | 0.673 ms |
| diagraph-layout 0.0.1: Graphviz `neato` | 1,335.096 ms | 1,077.089 ms |
| diagraph-layout 0.0.1: Graphviz `fdp` | 11,019.455 ms | 10,761.199 ms |
| diagraph-layout 0.0.1: Graphviz `sfdp` | 517.617 ms | 260.095 ms |

The API column is what the README reports. It includes the package's Typst input
encoding, WASM call, and result decoding. The WASM column isolates the `call
plugin` span inside that API call. Medians are calculated independently; their
difference is not an exact measurement of wrapper time.

The README groups related methods into two package columns, marking fast-layout
as ours. Stress SGD and majorization are compared with
[`neato`](https://graphviz.org/docs/layouts/neato/), which minimizes stress.
Spring is grouped with [`fdp`](https://graphviz.org/docs/layouts/fdp/) and its
multilevel alternative [`sfdp`](https://graphviz.org/docs/layouts/sfdp/).
These are algorithm families, not identical implementations or stopping rules.
Spectral and shell appear as additional, unpaired rows in the README. Buchheim
is listed as supported but unmeasured: it requires a tree, so this cyclic graph
cannot serve as its benchmark input. Unmeasured comparison cells do not imply
that Graphviz lacks related layout functionality.

## What is warmed

Each document calls one layout five times, with seeds 1 through 5. The first call
warms the loaded module and its code paths; only calls 2 through 5 enter either
median. Three fresh processes provide 12 warm samples per row. fast-layout uses
`seed`; Graphviz uses its [`start` attribute](https://graphviz.org/docs/attrs/start/).
Different request bytes prevent Typst from memoizing the WASM calls. The parser
requires exactly five API spans and five plugin calls in every process and checks
that each API span contains its corresponding plugin span.

The documents prepare graph inputs before calling the API. All calls must return
100 nodes; diagraph-layout must also report no error and return all 194 edges.
The documents produce the same blank one-page PDF. Package loading, graph input
construction, assertions, page layout, and PDF export are outside the measured
API spans. No baseline subtraction or fresh-process timing appears in the table.

Timings come from Typst's `--timings` instrumentation. The API span is the
outermost `func call` containing the single WASM export call. This includes the
work in both public wrappers and avoids measuring just the arithmetic engine.
Profiling itself can perturb timings, especially code with many Typst function
calls. Whole-document wall times and trace event counts are retained in the raw
record, but are not warm-call measurements.

A separate check compiled the same five-call documents without `--timings` for
stress SGD and all three Graphviz engines. Those whole-process wall times are
saved under `untraced_check` in the raw record. They include the first call and
are a sanity check only; they do not enter the README medians.

## What the comparison means

fast-layout computes numerical positions. Its default is dense all-pairs stress
SGD with 15 passes; majorization and spring default to 100 updates. All use the
package's default tolerance and two output dimensions. No speed-specific
options are applied to either package.

[diagraph-layout 0.0.1](https://typst.app/universe/package/diagraph-layout/)
exposes Graphviz layouts with node dimensions and edge routes. Its default node
sizes are retained and there are no labels. `neato`, `fdp`, and `sfdp` run with
their own default iteration limits and stopping rules, with an explicit seed.
They are useful alternatives for undirected graphs, but their outputs and
quality are not matched to fast-layout's numerical objectives. These results
establish package latency on this graph, not a universal solver ranking.

fast-layout is useful when the caller wants fast positions, SGD stress or spectral
layouts, pins, higher dimensions, and the same kernels in native Rust. CeTZ or
another renderer handles the drawing. Graphviz also offers hierarchical `dot`
layouts and routing features that fast-layout does not implement. Both Typst
packages separate layout from rendering.

## Reproduce

```sh
make pkgroot
python3 scripts/bench_warm.py --repeats 3 --calls 5
```

The first run downloads diagraph-layout 0.0.1 if needed, outside the measured
API spans. The script uses the bundled fast-layout WASM without rebuilding it.
Use `--diagraph-package /path/to/diagraph-layout/0.0.1` if the package cache is
outside the default Linux location; this path is used to record the plugin hash.

[Raw measurements](benchmarks/warm-comparison-100.json) include the exact graph,
Typst documents, all per-call samples, machine and compiler metadata, and WASM
hashes. Generated documents and full trace files remain under
`_bench/warm-comparison/`. The bundled WASM files are:

- fast-layout:0.1.0: 285,605 bytes; SHA-256 `55c735231dfdf15f3b45089150f318035459373bb4e8e5d0928c5b33c549a847`.
- diagraph-layout:0.0.1: 1,067,992 bytes; SHA-256 `05ff5801ff4c7bf32f2eac26464bb1330cab221bbef083f5d162ec02e1716df9`.
