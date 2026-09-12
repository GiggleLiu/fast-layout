#import "@preview/fast-layout:0.1.0": layout

#let edges = ((0, 1), (1, 2), (2, 3), (3, 4))
#let result = layout(5, edges, algorithm: "stress", seed: 7)
#let points = result.positions.map(p => (p.at(0) * 24pt, p.at(1) * 24pt))

#box(width: 240pt, height: 160pt, inset: 80pt)[
  #for edge in edges {
    line(start: points.at(edge.at(0)), end: points.at(edge.at(1)), stroke: 0.8pt)
  }
  #for point in points {
    place(dx: point.at(0), dy: point.at(1), circle(radius: 3pt, fill: black))
  }
]
