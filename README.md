# fast-layout

Graph layouts for Typst, computed in pure Rust. Supports stress, spring, spectral,
circular/shell, and Buchheim trees. Numerical layouts support 2D, 3D, and higher dimensions.

[Download the manual (PDF)](https://github.com/GiggleLiu/fast-layout/raw/refs/heads/main/fast-layout/manual.pdf)
· [API reference](fast-layout/README.md)

## Why another layout engine?

fast-layout focuses on fast node coordinates for custom drawings with CeTZ:
SGD stress, spectral layouts, configurable dimensions, and fixed coordinates.
The same kernels are also available as a native Rust library.

[diagraph-layout](https://typst.app/universe/package/diagraph-layout/) exposes
Graphviz, including hierarchical `dot` layouts, node sizes, and edge routing.
Choose it when you need those features; choose fast-layout when you want numerical
positions and control the drawing yourself. The warm-call comparison below
measures the cost of each package's layout API.

## Install

Version 0.1.0 is available from this repository. Install it locally:

```sh
git clone https://github.com/GiggleLiu/fast-layout.git
cd fast-layout
make install
```

Requires Typst 0.14.2 or newer. The compiled WASM is included; Rust is only
needed when rebuilding the engine.

## Draw a graph

Use fast-layout for coordinates and CeTZ to draw this 100-vertex, 180-edge grid:

```typst
#import "@preview/fast-layout:0.1.0": layout
#import "@preview/cetz:0.5.2"

#set page(width: auto, height: auto, margin: 12pt, fill: white)

#let edges = (
  range(100).filter(i => calc.rem(i, 10) < 9).map(i => (i, i + 1))
  + range(90).map(i => (i, i + 10))
)
#let result = layout(100, edges)
#let points = result.positions

#cetz.canvas(length: 6mm, {
  import cetz.draw: *
  for (a, b) in edges {
    line(points.at(a), points.at(b), stroke: 0.7pt + rgb("#94a3b8"))
  }
  for point in points {
    circle(point, radius: 2.5pt, fill: rgb("#2563eb"), stroke: none)
  }
})
```

![100-vertex grid drawn with CeTZ using the default stress layout](docs/graph-100.svg)

Node indices start at zero. Choose a layout with `algorithm`; use `dim` to set
its output dimension. Stress defaults to SGD with 15 passes.

## Performance

100 nodes and 194 edges, Typst 0.15.1, Intel Xeon Gold 6226R. Both packages use
**warm, uncached calls**: discard the first of five calls per process and change
the seed each time. Values are medians of 12 warm calls across three processes.
These are profiled **complete layout API calls**, including input encoding and
output decoding; plugin loading and drawing are excluded.

| Layout family | fast-layout 0.1.0 (ours) | diagraph-layout 0.0.1 (Graphviz) |
| --- | --- | --- |
| Stress minimization | SGD, default 15 passes: **21.4 ms**; majorization, 100 updates: **140.5 ms** | `neato`: **1,335.1 ms** |
| Force-directed | Spring, 100 updates: **47.0 ms** | `fdp`: **11,019.5 ms**; multilevel `sfdp`: **517.6 ms** |
| Spectral | **67.8 ms** | Not benchmarked |
| Circular / shell | **0.7 ms** | Not benchmarked |
| Buchheim tidy tree | Supported; not benchmarked | Not benchmarked |

Rows group related algorithms. diagraph-layout also computes node sizes and edge
routes; iteration budgets and final quality are not matched. "Not benchmarked"
means no warm measurement is available in this comparison, not that the package
lacks related functionality. Buchheim requires a tree and is not timed on this
cyclic benchmark graph.

[Benchmark details and reproduction](docs/warm-comparison.md)

## Acknowledgments

The original algorithms and tests are adapted from
[NetworkLayout.jl](https://github.com/JuliaGraphs/NetworkLayout.jl) by Abhijith
Anilkumar and contributors. Its [MIT license](fast-layout/NETWORKLAYOUT-LICENSE.md)
is retained. See [third-party notices](fast-layout/THIRD_PARTY-NOTICES.md) for all attribution.

## Development

See [building, testing, and preparing a submission](docs/development.md).
