#import "@preview/fast-layout:0.1.0": layout
#import "@preview/cetz:0.5.2"

#set page(width: 190mm, height: 245mm, margin: 14mm)
#set text(font: "Libertinus Serif", size: 9pt)

#let colors = (rgb("#2563eb"), rgb("#dc2626"), rgb("#059669"), rgb("#7c3aed"), rgb("#d97706"), rgb("#0891b2"))

#let draw-graph(title, result, edges, color: colors.at(0), project: p => (p.at(0), p.at(1))) = {
  let projected = result.positions.map(project)
  let xs = projected.map(p => p.at(0))
  let ys = projected.map(p => p.at(1))
  let min-x = calc.min(..xs)
  let max-x = calc.max(..xs)
  let min-y = calc.min(..ys)
  let max-y = calc.max(..ys)
  let width = 70mm
  let height = 38mm
  let scale = calc.min(
    (width - 12pt) / calc.max(max-x - min-x, 0.001),
    (height - 12pt) / calc.max(max-y - min-y, 0.001),
  )
  let midpoint = ((min-x + max-x) / 2, (min-y + max-y) / 2)
  let points = projected.map(p => (p.at(0) - midpoint.at(0), p.at(1) - midpoint.at(1)))

  block(width: 100%, inset: 8pt, stroke: 0.5pt + luma(210), radius: 4pt)[
    #text(weight: "bold", fill: color)[#title]
    #v(4pt)
    #box(width: width, height: height)[
      #align(center + horizon)[
        #cetz.canvas(length: scale, {
          import cetz.draw: *
          for edge in edges {
            line(points.at(edge.at(0)), points.at(edge.at(1)), stroke: 0.75pt + luma(155))
          }
          for (i, point) in points.enumerate() {
            circle(point, radius: 4.5pt, fill: color, stroke: 0.7pt + white)
            content(point, text(size: 6pt, fill: white, weight: "bold", str(i)))
          }
        })
      ]
    ]
  ]
}

#let graph-edges = (
  (0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0),
  (0, 3), (1, 4), (2, 6), (5, 7), (6, 7),
)
#let tree-edges = ((0, 1), (0, 2), (0, 3), (1, 4), (1, 5), (2, 6), (3, 7))
#let cycle-edges = ((0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 0))

= Layout gallery

Every panel computes coordinates with `fast-layout` and draws the result on a CeTZ canvas. Both axes use the same scale.

#grid(
  columns: (1fr, 1fr), gutter: 10pt, row-gutter: 10pt,
  draw-graph("Stress, majorization", layout(8, graph-edges, algorithm: "stress", seed: 7), graph-edges, color: colors.at(0)),
  draw-graph("Stress, SGD in 15 updates", layout(8, graph-edges, algorithm: "stress", stress-method: "sgd", iterations: 15, seed: 7), graph-edges, color: colors.at(1)),
  draw-graph("Spring", layout(8, graph-edges, algorithm: "spring", seed: 7, iterations: 180), graph-edges, color: colors.at(2)),
  draw-graph("Spectral", layout(8, cycle-edges, algorithm: "spectral"), cycle-edges, color: colors.at(3)),
  draw-graph("Shell", layout(8, graph-edges, algorithm: "shell", shells: ((0,), (1, 2, 3))), graph-edges, color: colors.at(4)),
  draw-graph("Buchheim tree", layout(8, tree-edges, algorithm: "buchheim", node-sizes: (1.0, 1.4, 0.8)), tree-edges, color: colors.at(5)),
)

#pagebreak()

== A three-dimensional layout on a page

`dim` remains configurable for the numerical algorithms. This panel computes three coordinates, then applies a short oblique projection only for drawing.

#align(center)[
  #draw-graph(
    "Spectral, dim: 3",
    layout(8, graph-edges, algorithm: "spectral", dim: 3),
    graph-edges,
    color: colors.at(5),
    project: p => (p.at(0) - 0.45 * p.at(2), p.at(1) - 0.3 * p.at(2)),
  )
]
