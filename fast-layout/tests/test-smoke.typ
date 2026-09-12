#import "@preview/fast-layout:0.1.0": layout

#let result = layout(4, ((0, 1), (1, 2), (2, 3)), algorithm: "stress")
#assert.eq(result.positions.len(), 4)
#assert(result.positions.all(position => position.len() == 2))
#assert(result.iterations <= 100)
Engine layout smoke test.
