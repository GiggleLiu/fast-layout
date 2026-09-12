#import "@preview/fast-layout:0.1.0": layout
#import "@preview/cetz:0.5.2"

#set page(width: 150mm, height: 100mm, margin: 14mm)
#set text(font: "Libertinus Serif", size: 10pt)

#let edges = ((0, 1), (1, 2), (2, 3), (3, 4))
#let result = layout(5, edges, algorithm: "stress", seed: 7)

#let draw-graph(result, edges, width: 96mm, height: 48mm) = {
  let xs = result.positions.map(p => p.at(0))
  let ys = result.positions.map(p => p.at(1))
  let min-x = calc.min(..xs)
  let max-x = calc.max(..xs)
  let min-y = calc.min(..ys)
  let max-y = calc.max(..ys)
  let scale = calc.min(
    (width - 12pt) / calc.max(max-x - min-x, 0.001),
    (height - 12pt) / calc.max(max-y - min-y, 0.001),
  )
  let midpoint = ((min-x + max-x) / 2, (min-y + max-y) / 2)
  let points = result.positions.map(p => (p.at(0) - midpoint.at(0), p.at(1) - midpoint.at(1)))

  box(width: width, height: height)[
    #align(center + horizon)[
      #cetz.canvas(length: scale, {
        import cetz.draw: *
        for edge in edges {
          line(points.at(edge.at(0)), points.at(edge.at(1)), stroke: 0.8pt + luma(145))
        }
        for (i, point) in points.enumerate() {
          circle(point, radius: 5pt, fill: rgb("#2563eb"), stroke: 0.7pt + white)
          content(point, text(size: 6.5pt, fill: white, weight: "bold", str(i)))
        }
      })
    ]
  ]
}

= A path laid out with fast-layout and drawn with CeTZ

#draw-graph(result, edges)
