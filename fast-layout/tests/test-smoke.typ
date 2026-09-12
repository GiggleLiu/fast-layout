#import "@preview/fast-layout:0.1.0": layout

#let edges = ((0, 1), (1, 2), (2, 3))
#let result = layout(4, edges, tolerance: 0.0)
#assert.eq(result.positions.len(), 4)
#assert(result.positions.all(position => position.len() == 2))
#assert.eq(result.iterations, 15)
#assert.eq(result, layout(4, edges, stress-method: "sgd", iterations: 15, tolerance: 0.0))
#assert.eq(layout(4, edges, iterations: 7, tolerance: 0.0).iterations, 7)
#assert.eq(layout(4, edges, algorithm: "spring", tolerance: 0.0).iterations, 100)
#assert.eq(layout(4, edges, stress-method: "majorization", tolerance: 0.0).iterations, 100)
Engine layout smoke test.
