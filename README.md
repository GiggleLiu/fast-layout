# fast-layout

Graph layouts for Typst, computed in pure Rust. Supports stress, spring, spectral,
circular/shell, and Buchheim trees. Numerical layouts support 2D, 3D, and higher dimensions.

[Download the manual (PDF)](https://github.com/GiggleLiu/fast-layout/raw/refs/heads/main/fast-layout/manual.pdf)
· [API reference](fast-layout/README.md)

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
#let result = layout(4, edges, stress-method: "sgd", iterations: 15)
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
its output dimension. Stress defaults to majorization; the example selects SGD
with 15 passes for faster computation.

## Performance

100 nodes, Typst 0.15.1, Intel Xeon Gold 6226R. Median plugin-call time over
three runs; drawing is excluded. Warm calls reuse the loaded plugin and compute
a new layout.

| Layout | First call | Warm call |
| --- | ---: | ---: |
| Stress SGD, 15 passes | 28.5 ms | 21.0 ms |
| Stress majorization, 100 updates | 147.2 ms | 139.9 ms |
| Spring, 100 updates | 53.1 ms | 46.9 ms |
| Spectral | 75.2 ms | 67.7 ms |
| Shell | 4.7 ms | 0.5 ms |

[Benchmark details](docs/performance-100.md)

## Acknowledgments

The original algorithms and tests are adapted from
[NetworkLayout.jl](https://github.com/JuliaGraphs/NetworkLayout.jl) by Abhijith
Anilkumar and contributors. Its [MIT license](fast-layout/NETWORKLAYOUT-LICENSE.md)
is retained. See [third-party notices](THIRD_PARTY-NOTICES.md) for all attribution.
