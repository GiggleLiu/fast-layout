# Full-render comparison with diagraph

This benchmark compares complete Typst documents on the same deterministic,
connected undirected graphs. Each graph has 100, 500, or 1000 nodes and about
twice as many edges. The source generator is `scripts/bench_typst.py`; raw
samples are in [`benchmarks/diagraph-comparison.json`](benchmarks/diagraph-comparison.json).

`fast-layout` computes coordinates, then the document draws every edge as a
Typst `line` and every node as an unlabeled `circle`. diagraph 0.3.7 sends the
same nodes and edges as an undirected DOT graph to Graphviz and returns its
finished SVG. Its package API and version are documented on the
[Typst Universe page](https://typst.app/universe/package/diagraph/). The
[0.3.7 render implementation](https://github.com/typst/packages/blob/main/packages/preview/diagraph/0.3.7/src/internals.typ#L365-L425)
shows the Graphviz engine selection and SVG conversion used here.

## Median fresh compile time

| Nodes | fast-layout stress + Typst drawing | fast-layout spring + Typst drawing | diagraph neato render | diagraph sfdp render |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 0.223 s | 0.199 s | 1.627 s | 0.832 s |
| 500 | 5.202 s | 1.900 s | 20 s timeout | 5.408 s |
| 1000 | 20 s timeout | 4.264 s | 20 s timeout | 15.927 s |

The coordinate-only cases remain separate in the JSON. Their medians were
0.220/0.192 s at 100 nodes, 4.979/1.851 s at 500 nodes, and timeout/4.318 s at
1000 nodes for stress/spring. They should not be compared directly with
diagraph's full SVG renderer.

## Method

The command was:

```sh
python3 scripts/bench_typst.py \
  --sizes 100,500,1000 \
  --algorithms stress,spring \
  --repeats 3 \
  --timeout 20 \
  --compare-render \
  --output docs/benchmarks/diagraph-comparison.json
```

Each sample starts a fresh `typst compile` process. The script warms Typst and
downloads and initializes diagraph before timing. It writes the JSON after
every case, so a later timeout or interruption does not discard completed
cases.

The machine used an Intel Xeon Gold 6226R at 2.90 GHz, Linux 7.0.0-30 x86-64,
Typst 0.15.1, and Python 3.12.8. The fast-layout WASM SHA-256 was
`12c5351b62ab808cf3dd8ad9b92a0f61c090050a7a67ee273457d796ae9b2d08`.
fast-layout used its defaults: seed 1, 100 maximum iterations, tolerance
`1e-5`, and automatic spring approximation settings. diagraph 0.3.7 bundles
Graphviz 14.1.4; neato and sfdp used Graphviz's package defaults because their
iteration and cooling controls do not map directly to fast-layout's controls.

These are document workload timings, not matched optimizer benchmarks.
Graphviz routes edges and produces SVG paths. The native Typst renderer draws
straight lines and circles. Node labels are empty in both outputs, but node
sizing, edge routing, normalization, convergence, and cooling still differ.
Post-run compiles of every successful diagraph engine/size combination produced
nonempty PDFs with no extracted Graphviz error text; the 500- and 1000-node
neato cases never completed within the limit.
