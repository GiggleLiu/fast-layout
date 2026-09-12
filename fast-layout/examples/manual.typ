#import "@preview/fast-layout:0.1.0": layout

#set page(width: 190mm, height: 245mm, margin: 14mm)
#set text(size: 9pt)

#let palette = (rgb("#2563eb"), rgb("#dc2626"), rgb("#059669"), rgb("#7c3aed"), rgb("#d97706"), rgb("#0891b2"))

#let plot(title, result, edges, color: palette.at(0), project: p => (p.at(0), -p.at(1))) = {
  let projected = result.positions.map(project)
  let xs = projected.map(p => p.at(0))
  let ys = projected.map(p => p.at(1))
  let min-x = calc.min(..xs)
  let min-y = calc.min(..ys)
  let span-x = calc.max(calc.max(..xs) - min-x, 0.001)
  let span-y = calc.max(calc.max(..ys) - min-y, 0.001)
  let points = projected.map(p => (
    (8 + 104 * (p.at(0) - min-x) / span-x) * 1pt,
    (8 + 78 * (p.at(1) - min-y) / span-y) * 1pt,
  ))

  block(width: 100%, inset: 8pt, stroke: 0.5pt + luma(210), radius: 4pt)[
    #text(weight: "bold", fill: color)[#title]
    #v(5pt)
    #box(width: 120pt, height: 94pt)[
      #for edge in edges {
        place(
          line(
            start: points.at(edge.at(0)),
            end: points.at(edge.at(1)),
            stroke: 0.8pt + luma(150),
          ),
        )
      }
      #for (i, point) in points.enumerate() {
        place(
          dx: point.at(0), dy: point.at(1),
          circle(radius: 5pt, fill: color, stroke: 0.7pt + white),
        )
        place(dx: point.at(0) - 2.5pt, dy: point.at(1) - 3.6pt, text(size: 6pt, fill: white, str(i)))
      }
    ]
  ]
}

#let graph-edges = (
  (0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0),
  (0, 3), (1, 4), (2, 6), (5, 7), (6, 7),
)
#let tree-edges = ((0, 1), (0, 2), (0, 3), (1, 4), (1, 5), (2, 6), (3, 7))
#let cycle-edges = ((0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 0))

= Graph layout gallery

Each algorithm returns coordinates in node order. The drawing below uses only Typst's native `line`, `circle`, and `place` primitives.

#grid(
  columns: (1fr, 1fr),
  gutter: 10pt,
  row-gutter: 10pt,
  plot("Stress", layout(8, graph-edges, algorithm: "stress", seed: 7), graph-edges, color: palette.at(0)),
  plot("Spring", layout(8, graph-edges, algorithm: "spring", seed: 7, iterations: 180), graph-edges, color: palette.at(1)),
  plot("Spectral", layout(8, cycle-edges, algorithm: "spectral"), cycle-edges, color: palette.at(2)),
  plot("Shell", layout(8, graph-edges, algorithm: "shell", shells: ((0,), (1, 2, 3))), graph-edges, color: palette.at(3)),
  plot("Buchheim tree", layout(8, tree-edges, algorithm: "buchheim", node-sizes: (1.0, 1.4, 0.8)), tree-edges, color: palette.at(4)),
  plot(
    "Spectral 3D → 2D",
    layout(8, graph-edges, algorithm: "spectral", dim: 3),
    graph-edges,
    color: palette.at(5),
    project: p => (p.at(0) - 0.45 * p.at(2), p.at(1) - 0.3 * p.at(2)),
  ),
)

The last panel computes a numerical three-dimensional layout, then applies a simple oblique projection for drawing.
