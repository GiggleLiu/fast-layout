#import "@preview/fast-layout:0.1.0" as fast-layout
#set page(width: 460pt, margin: 28pt, height: auto)
#set text(size: 10pt)

= fast-layout manual (v#fast-layout.fast-layout-version)

`fast-layout` computes coordinates and leaves drawing to Typst or a drawing
package. Node indices start at zero, and position `i` belongs to node `i`.

== Basic use

#let edges = ((0, 1), (1, 2), (2, 3), (3, 0), (0, 2))
#let result = fast-layout.layout(4, edges, algorithm: "stress", seed: 4)
#let xs = result.positions.map(p => p.at(0))
#let ys = result.positions.map(p => p.at(1))
#let min-x = calc.min(..xs)
#let min-y = calc.min(..ys)
#let span-x = calc.max(calc.max(..xs) - min-x, 0.001)
#let span-y = calc.max(calc.max(..ys) - min-y, 0.001)
#let points = result.positions.map(p => (
  (5 + 175 * (p.at(0) - min-x) / span-x) * 1pt,
  (5 + 95 * (p.at(1) - min-y) / span-y) * 1pt,
))

#box(width: 220pt, height: 130pt, inset: 15pt, stroke: 0.5pt + luma(210), radius: 4pt)[
  #for edge in edges {
    place(line(start: points.at(edge.at(0)), end: points.at(edge.at(1))))
  }
  #for (i, point) in points.enumerate() {
    place(
      dx: point.at(0),
      dy: point.at(1),
      circle(radius: 7pt, fill: white, stroke: black)[#i],
    )
  }
]

The result also reports `iterations`, `converged`, and an optional `objective`.
Available algorithms are `stress`, `spring`, `spectral`, `shell`, and
`buchheim`. `circular` is an alias for `shell`.

Stress and spring support initial positions and per-coordinate pins. Spectral
uses edge affinities and optional node weights. Shell accepts groups through
`shells`. Buchheim treats edges as parent-to-child links and accepts node
sizes plus a root index.
