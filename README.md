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

## Draw a graph

Use fast-layout for coordinates and CeTZ for drawing:

```typst
#import "@preview/fast-layout:0.1.0": layout
#import "@preview/cetz:0.5.2"

#let edges = ((0, 1), (1, 2), (2, 3), (3, 0))
#let result = layout(4, edges)
#let points = result.positions

#cetz.canvas(length: 1cm, {
  import cetz.draw: *
  for (a, b) in edges {
    line(points.at(a), points.at(b))
  }
  for point in points {
    circle(point, radius: 3pt, fill: white)
  }
})
```

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

Rows group related algorithms. diagraph-layout also computes node sizes and edge
routes; iteration budgets and final quality are not matched. Spectral and shell
timings are listed separately in the detailed report, without a paired Graphviz
comparison.

[Benchmark details and reproduction](docs/warm-comparison.md)

## Acknowledgments

The original algorithms and tests are adapted from
[NetworkLayout.jl](https://github.com/JuliaGraphs/NetworkLayout.jl) by Abhijith
Anilkumar and contributors. Its [MIT license](fast-layout/NETWORKLAYOUT-LICENSE.md)
is retained. See [third-party notices](THIRD_PARTY-NOTICES.md) for all attribution.
