#import "@preview/fast-layout:0.1.0" as fast-layout
#import "@preview/cetz:0.5.2"

#set page(paper: "a4", margin: (x: 19mm, y: 17mm), numbering: "1")
#set text(font: "Libertinus Serif", size: 9.5pt)
#set heading(numbering: "1.1")
#show raw.where(block: true): set block(fill: luma(246), inset: 7pt, radius: 3pt)

#let edges = ((0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0), (0, 3), (1, 4), (2, 6), (5, 7), (6, 7))
#let tree-edges = ((0, 1), (0, 2), (0, 3), (1, 4), (1, 5), (2, 6), (3, 7))
#let cycle-edges = ((0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 0))

#let draw-graph(result, graph-edges, width: 71mm, height: 43mm, color: rgb("#2563eb"), project: p => (p.at(0), p.at(1))) = {
  let projected = result.positions.map(project)
  let xs = projected.map(p => p.at(0))
  let ys = projected.map(p => p.at(1))
  let min-x = calc.min(..xs)
  let max-x = calc.max(..xs)
  let min-y = calc.min(..ys)
  let max-y = calc.max(..ys)
  let scale = calc.min(
    (width - 13pt) / calc.max(max-x - min-x, 0.001),
    (height - 13pt) / calc.max(max-y - min-y, 0.001),
  )
  let midpoint = ((min-x + max-x) / 2, (min-y + max-y) / 2)
  let points = projected.map(p => (p.at(0) - midpoint.at(0), p.at(1) - midpoint.at(1)))

  box(width: width, height: height, stroke: 0.5pt + luma(215), radius: 3pt)[
    #align(center + horizon)[
      #cetz.canvas(length: scale, {
        import cetz.draw: *
        for edge in graph-edges {
          line(points.at(edge.at(0)), points.at(edge.at(1)), stroke: 0.75pt + luma(145))
        }
        for (i, point) in points.enumerate() {
          circle(point, radius: 4.5pt, fill: color, stroke: 0.7pt + white)
          content(point, text(size: 6pt, fill: white, weight: "bold", str(i)))
        }
      })
    ]
  ]
}

#let pair(code, figure) = block(breakable: false)[
  #grid(columns: (1fr, 1fr), gutter: 10pt, align: horizon, code, align(center + horizon, figure))
]

= fast-layout

#text(fill: luma(90))[Version #fast-layout.fast-layout-version]

`fast-layout` computes graph coordinates. CeTZ turns those coordinates into lines and nodes. Keeping these jobs separate lets the same layout drive labels, custom shapes, or another drawing package.

Node indices start at zero. Position `i` belongs to node `i`, including isolated nodes.

== First layout

#let first = fast-layout.layout(8, edges, algorithm: "stress", seed: 7)
#pair([
```typst
#let result = layout(
  8, edges,
  algorithm: "stress",
  seed: 7,
)
```
], draw-graph(first, edges))

The default stress method is majorization with at most 100 updates. The result contains `positions`, `iterations`, `converged`, and `objective`.

== Choose a stress method

Majorization uses constrained global solves and rejects updates that increase stress beyond roundoff. SGD visits shuffled node pairs with a decaying step schedule. It can reach a lower objective on some graphs, but it does not enforce a monotonic global objective. Fifteen SGD updates are a useful low-cost recipe.

#let major = fast-layout.layout(8, edges, algorithm: "stress", seed: 7)
#let quick = fast-layout.layout(8, edges, algorithm: "stress", stress-method: "sgd", iterations: 15, seed: 7)
#grid(
  columns: (1fr, 1fr), gutter: 10pt,
  [*Default majorization* #v(3pt) #draw-graph(major, edges, width: 72mm, height: 38mm)],
  [*15-update SGD* #v(3pt) #draw-graph(quick, edges, width: 72mm, height: 38mm, color: rgb("#dc2626"))],
)

```typst
#let quick = layout(
  8, edges,
  algorithm: "stress",
  stress-method: "sgd",
  iterations: 15,
)
```

Both methods use edge weights as target lengths. A weight of `2` asks for twice the path length of a weight of `1`.

#pagebreak()

== The five algorithms

=== Stress

Stress fits Euclidean distances to weighted shortest-path distances. It accepts initial coordinates and per-coordinate pins. Majorization and SGD optimize the same objective with different update rules.

=== Spring

Spring uses Fruchterman-Reingold attraction and repulsion. Edge weights do not change its force. Edges only state connectivity. For 2D and 3D graphs of at least 256 nodes, the default switches to Barnes-Hut approximation. Set `theta: 0` for exact forces.

#pair([
```typst
#layout(
  8, edges,
  algorithm: "spring",
  iterations: 180,
  seed: 7,
)
```
], draw-graph(fast-layout.layout(8, edges, algorithm: "spring", iterations: 180, seed: 7), edges, color: rgb("#059669")))

=== Spectral

Spectral treats edge weights as affinities. Larger values pull endpoints into the same low-frequency modes. Optional node weights also act as affinities. Eigenvector signs can flip, so compare distances rather than raw coordinates across engines.

#pair([
```typst
#layout(
  8, cycle-edges,
  algorithm: "spectral",
)
```
], draw-graph(fast-layout.layout(8, cycle-edges, algorithm: "spectral"), cycle-edges, color: rgb("#7c3aed")))

=== Shell

Shell places explicit groups on circles, from inner to outer. Any unlisted nodes form the last shell. The alias `"circular"` selects the same algorithm.

#pair([
```typst
#layout(
  8, edges,
  algorithm: "shell",
  shells: ((0,), (1, 2, 3)),
)
```
], draw-graph(fast-layout.layout(8, edges, algorithm: "shell", shells: ((0,), (1, 2, 3))), edges, color: rgb("#d97706")))

#pagebreak()

=== Buchheim

Buchheim lays out a rooted, ordered tree. Edges point from parent to child, and input order sets sibling order. `node-sizes` contains diameters; omitted entries use `1`.

#pair([
```typst
#layout(
  8, tree-edges,
  algorithm: "buchheim",
  node-sizes: (1.0, 1.4, 0.8),
)
```
], draw-graph(fast-layout.layout(8, tree-edges, algorithm: "buchheim", node-sizes: (1.0, 1.4, 0.8)), tree-edges, color: rgb("#0891b2")))

== Initial positions and pins

Stress and spring accept `initial` positions. Use `none` for a node that should keep its seeded start. A pin mask fixes each coordinate independently.

```typst
#let result = layout(
  3, ((0, 1), (1, 2)),
  initial: ((0, 0), (2, 1), none),
  pins: ((true, true), (true, false)),
  edge-weights: (2, 1),
)
```

Here node 0 cannot move. Node 1 keeps its x coordinate but may move along y. A pinned coordinate without an explicit initial value stays at its seeded value.

== Dimensions and projection

Stress, spring, and spectral return `dim` coordinates per node. The default is 2, and the accepted range is 1 through 4096. Shell and Buchheim are always 2D.

#let three-d = fast-layout.layout(8, edges, algorithm: "spectral", dim: 3)
#pair([
```typst
#let result = layout(
  8, edges,
  algorithm: "spectral",
  dim: 3,
)
#let project = p => (
  p.at(0) - 0.45 * p.at(2),
  p.at(1) - 0.30 * p.at(2),
)
```
], draw-graph(three-d, edges, color: rgb("#0891b2"), project: p => (p.at(0) - 0.45 * p.at(2), p.at(1) - 0.30 * p.at(2))))

The projection belongs to the drawing code. The returned positions still contain all three coordinates.

== Main options

#table(
  columns: (30mm, 32mm, 1fr), inset: 2.5pt, stroke: 0.35pt + luma(220),
  table.header([*Option*], [*Default*], [*Meaning*]),
  [`algorithm`], [`"stress"`], [Layout algorithm],
  [`stress-method`], [`"majorization"`], [`majorization` or `sgd`; stress only],
  [`dim`], [`2`], [Coordinates per node],
  [`seed`], [`1`], [Initialization, collision handling, and SGD pair order],
  [`iterations`], [`100`], [Maximum numerical updates],
  [`tolerance`], [`1e-5`], [Stopping threshold],
  [`initial`, `pins`], [`()`], [Starts and coordinate masks for stress or spring],
  [`edge-weights`], [`()`], [Stress target lengths or spectral affinities],
  [`theta`, `c`, `temperature`], [`none`], [Spring approximation, spacing, and movement cap],
  [`node-weights`], [`()`], [Spectral node affinities],
  [`shells`], [`()`], [Inner-to-outer node groups],
  [`node-sizes`, `root`], [`()`, `0`], [Buchheim diameters and root],
)

#pagebreak()

== Results and stopping

`iterations` counts numerical updates after initialization. `converged` says whether the method met its tolerance before the limit. SGD may stop after its schedule no longer clips pair steps and the largest pair-endpoint movement meets `tolerance`. With `tolerance: 0`, runs with movable coordinates use the requested budget and report `converged: false`. Trivial numerical layouts and fully pinned stress layouts converge without using the budget.

Stress reports weighted stress. Spring reports exact spring energy for exact-force runs and `none` for Barnes-Hut runs. The other algorithms report `none`.

For stress and spring, `tolerance: 0` disables early stopping. Sparse spectral iteration checks eigenpair residuals. Shell, Buchheim, and trivial numerical layouts complete without iterative updates.

All requests are deterministic for a fixed seed and engine build. Floating-point results can differ slightly across platforms.

== Drawing with CeTZ

CeTZ is a document dependency, not part of the layout engine. Pass positions into `cetz.canvas` and use a single scale for both axes.

```typst
#import "@preview/fast-layout:0.1.0": layout
#import "@preview/cetz:0.5.2"

#let edges = ((0, 1), (1, 2), (2, 3), (3, 0))
#let result = layout(4, edges, algorithm: "stress")
#let xs = result.positions.map(p => p.at(0))
#let ys = result.positions.map(p => p.at(1))
#let min-x = calc.min(..xs)
#let max-x = calc.max(..xs)
#let min-y = calc.min(..ys)
#let max-y = calc.max(..ys)
#let scale = calc.min(
  90mm / calc.max(max-x - min-x, 0.001),
  45mm / calc.max(max-y - min-y, 0.001),
)
#let midpoint = (
  (min-x + max-x) / 2,
  (min-y + max-y) / 2,
)
#let points = result.positions.map(p => (
  p.at(0) - midpoint.at(0),
  p.at(1) - midpoint.at(1),
))

#cetz.canvas(length: scale, {
  import cetz.draw: *
  for edge in edges {
    line(points.at(edge.at(0)), points.at(edge.at(1)))
  }
  for point in points {
    circle(point, radius: 4pt, fill: white)
  }
})
```

The examples choose the smaller available-width and available-height ratio. That preserves angles and relative distances instead of stretching each axis separately.

== Limits and errors

Invalid indices, incompatible options, non-finite values, and malformed trees return errors. Stress and spectral accept at most 4096 nodes. The general request boundary also caps nodes, edges, output coordinates, and encoded input size to prevent unreasonable allocations. These caps do not promise interactive runtime near the limit.

Use spring for large 2D or 3D graphs. For weighted-distance layouts, compare the default majorization method with the 15-update SGD recipe on the graphs that matter to your document.
