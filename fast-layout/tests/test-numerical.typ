#import "@preview/fast-layout:0.1.0": layout

#let close(a, b, tolerance: 1e-8) = assert(calc.abs(a - b) <= tolerance, message: "expected " + str(a) + " ≈ " + str(b))
#let point-close(actual, expected, tolerance: 1e-8) = {
  assert.eq(actual.len(), expected.len())
  for (a, b) in actual.zip(expected) { close(a, b, tolerance: tolerance) }
}
#let distance(a, b) = calc.sqrt(a.zip(b).map(pair => calc.pow(pair.at(0) - pair.at(1), 2)).sum())

// Julia NetworkLayout.jl spring fixture, using exact forces and explicit initialization.
#let spring = layout(
  5,
  ((0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (0, 2)),
  algorithm: "spring",
  initial: ((-1.0, 0.2), (-0.3, 0.8), (0.4, -0.7), (1.1, 0.1), (0.2, 1.3)),
  iterations: 8,
  tolerance: 0.0,
  theta: 0.0,
  c: 2.0,
  temperature: 2.0,
)
#let spring-expected = (
  (-1.565989815349752, -0.6115860655569684),
  (-2.668383570202298, 0.15863242521582097),
  (-0.41163623573867325, -1.005085054144798),
  (1.8119862173474548, -0.2273229044584045),
  (0.11794233468937834, 1.0326097338541904),
)
#for (actual, expected) in spring.positions.zip(spring-expected) { point-close(actual, expected) }

// A weighted path already at its ideal edge lengths is a zero-stress solution.
#let stress = layout(
  3,
  ((0, 1), (1, 2)),
  algorithm: "stress",
  edge-weights: (1.0, 2.0),
  initial: ((-1.3333333333333333, 0.0), (-0.3333333333333333, 0.0), (1.6666666666666667, 0.0)),
  iterations: 2,
  tolerance: 0.0,
)
#close(distance(stress.positions.at(0), stress.positions.at(1)), 1.0)
#close(distance(stress.positions.at(1), stress.positions.at(2)), 2.0)
#close(distance(stress.positions.at(0), stress.positions.at(2)), 3.0)

// A four-cycle's spectral embedding is a square, independent of rotation or reflection.
#let spectral = layout(4, ((0, 1), (1, 2), (2, 3), (3, 0)), algorithm: "spectral")
#let side = distance(spectral.positions.at(0), spectral.positions.at(1))
#close(distance(spectral.positions.at(1), spectral.positions.at(2)), side)
#close(distance(spectral.positions.at(2), spectral.positions.at(3)), side)
#close(distance(spectral.positions.at(3), spectral.positions.at(0)), side)
#close(distance(spectral.positions.at(0), spectral.positions.at(2)), calc.sqrt(2) * side)

// Pins are exact coordinate constraints in higher dimensions.
#let pinned = layout(
  2,
  ((0, 1),),
  algorithm: "spring",
  dim: 3,
  initial: ((3.0, -2.0, 1.5), (0.0, 0.0, 0.0)),
  pins: ((true, true, true), ()),
  iterations: 5,
)
#assert.eq(pinned.positions.at(0), (3.0, -2.0, 1.5))

// Directed root selection and depth spacing for Buchheim trees.
#let tree = layout(4, ((2, 0), (2, 1), (1, 3)), algorithm: "buchheim", root: 2)
#assert.eq(tree.positions, ((-1.0, -2.0), (1.0, -2.0), (0.0, 0.0), (1.0, -4.0)))

Numerical layout checks passed.
