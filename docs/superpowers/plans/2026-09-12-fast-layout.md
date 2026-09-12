# Graph layout implementation plan

Date: 2026-09-12
Status: Implemented and reviewed; validation and performance records complete, with additional profiling coverage deferred below.
Working project name: `fast-layout`.

## Goal and agreed scope

Build a fast, pure Rust fast-layout library with a bundled Typst WASM plugin,
following the project structure and development commands in `../chalks/`.
Port the selected NetworkLayout.jl algorithms and their behavioral tests before
optimizing them. Measure layout quality as well as runtime.

| Algorithm | Dimensions | Priority |
| --- | --- | --- |
| Stress majorization | Configurable, including 2D and 3D | First; essential |
| Spring / Fruchterman–Reingold | Configurable, including 2D and 3D | First |
| Spectral | Configurable, including 2D and 3D | Included |
| Circular / shell | Natural 2D coordinates | Included |
| Buchheim tidy tree | Natural 2D coordinates | Included |

Keep weighted graphs where meaningful, reproducible seeds, initial positions,
and per-coordinate pins for stress and spring. Handle disconnected graphs,
isolated vertices, and empty graphs explicitly. Preserve node order in results.

Defer SFDP, square grid, alignment, animation/iteration-history APIs, alternate
numeric types, graph rendering, and edge routing. Do not restrict the numerical
algorithms to 2D. Optimize 2D and 3D without making those the only valid dimensions.

## Project structure

```text
fast-layout/
├── Cargo.toml / Cargo.lock
├── rust-toolchain.toml
├── Makefile
├── README.md / LICENSE / THIRD_PARTY-NOTICES.md
├── .github/workflows/ci.yml
├── fast-layout-engine/
│   ├── Cargo.toml
│   ├── src/                 # graph, numerical algorithms, validation, CBOR bridge
│   ├── tests/               # translated Julia tests and numerical fixtures
│   └── benches/             # native runtime and quality measurements
├── fast-layout/
│   ├── typst.toml / lib.typ / Makefile
│   ├── src/engine.typ
│   ├── plugin/fast_layout_engine.wasm
│   ├── tests/               # successful compilations and expected errors
│   ├── examples/ / manual.typ
│   └── LICENSE / NETWORKLAYOUT-LICENSE.md
├── upstream/NetworkLayout.jl/
├── scripts/                 # reference generation and Typst benchmarking
└── docs/superpowers/plans/2026-09-12-fast-layout.md
```

Use Rust 1.93.1, `wasm32-unknown-unknown`, `cdylib` plus `rlib`, and the
`wasm-minimal-protocol` + `serde` + `ciborium` bridge used by chalks. Choose one
pure Rust linear-algebra dependency after a small native/WASM compatibility and
performance check. Do not depend on Julia, Python, Graphviz, BLAS, or LAPACK at
runtime. Julia is only a development reference.

Use a locked dependency graph and a reproducible WASM build with path remapping.
Bundle the WASM in the Typst package. Package publishing and creating a remote
repository are separate from this implementation.

## API contract

- The Rust API accepts a node count, indexed edges, and algorithm-specific
  options; it returns coordinates without serialization overhead.
- The Typst API wraps the same computation with one CBOR request and response.
  The WASM exports `version` and `layout`.
- Node indices are zero-based. Output position `i` always belongs to node `i`.
  An explicit node count preserves isolated nodes.
- Use sparse graph storage and contiguous coordinate buffers. Do not allocate
  an adjacency matrix for algorithms that only need adjacency lists.
- Positions contain `dim` finite `f64` coordinates. Dimensions and all initial
  positions and pin masks must agree. Reject invalid indices, unknown options,
  incompatible options, and non-finite numeric input with useful errors.
- Edge weights have algorithm-specific meanings: target lengths for stress,
  affinities for spectral. Document spring's treatment of weights instead of
  silently assigning the same meaning to all algorithms.
- Define duplicate-edge, self-loop, and directed-input behavior explicitly.
  Buchheim consumes parent-to-child edges and requires a valid rooted tree;
  it must reject cycles, multiple parents, and unreachable vertices.
- Stress and spring accept reproducible initial positions and per-coordinate
  pins. Pins stay exact; postprocessing must not move them.
- Report iterations and convergence status for numerical algorithms. A work
  limit must not be reported as successful convergence.
- Determinism means identical requests produce identical results with the same
  engine build. Compare native versus WASM and Julia numerically, not by bytes.

Typst usage, validated during implementation:

```typst
#import "@preview/fast-layout:0.1.0": layout

#let result = layout(
  4,
  ((0, 1), (1, 2), (2, 3)),
  algorithm: "stress",
  dim: 3,
  seed: 42,
)
// result.positions has four three-coordinate positions.
```

## Task 1: Preserve the reference and establish test cases

- [x] Record NetworkLayout.jl version 0.4.10 and commit
  `073192ac737ff3309d7a1204fdd99ea361232d2b` with source URL and license notices.
  Complete the reference snapshot needed to run it, including its extension.
- [x] Keep the upstream source and tests unchanged. Document local corrections
  separately; do not edit reference tests to accommodate Rust results.
- [x] Map every test for the five selected algorithms to a Rust test or an
  explicit exclusion. Julia-specific constructors, macros, and numeric-type
  inference do not require imitation in Rust. Deferred algorithms are marked
  out of scope, not counted as passing.
- [x] Translate wheel, jagmesh, weighted-distance, disconnected-graph, pin,
  shell, and tree cases. Include the original jagmesh fixture and provenance.
- [x] Generate numerical fixtures by actually running Julia with explicit
  initial positions and options. Avoid relying on matching Julia's RNG.
- [x] Audit Julia's iteration counting and returned iterate against its
  iterator. Record intentional bug fixes and count updates explicitly in Rust.
- [x] Record initial native Julia and Typst/diagraph-layout timings on the same
  machine and fixed inputs. Set concrete performance targets from that baseline
  before optimizing; record the graph, dimensions, iterations, and quality target.

Deliverable: a reference manifest, test-port map, reproducible fixtures, and
baseline benchmark results. Tests must assert mathematical behavior, not merely
that a vector of the right size was returned.

## Task 2: Rust crate, graph input, and WASM smoke test

- [x] Create the Cargo workspace and chalks-style Makefiles and local package
  resolution under `_pkgroot`.
- [x] Add the sparse graph representation, dimension-aware coordinate storage,
  deterministic initialization, and trust-boundary validation.
- [x] Prove the numerical dependency works on both native Rust and Typst's
  actual WASM runtime, with no external runtime imports.
- [x] Test the CBOR version/request/response path, malformed requests, and
  stable output order. Keep graph drawing outside the package.

Deliverable: `make rust-test`, `make plugin`, and the Typst bridge smoke test.

## Task 3: Stress majorization first

- [x] Port the weighted stress objective, shortest-path distances, majorization
  update, initial positions, convergence tolerances, and pin behavior tests.
- [x] Use repeated BFS for unweighted shortest paths and Dijkstra for positive
  weighted paths, avoiding an unconditional cubic Floyd–Warshall pass.
- [x] Replace Julia's explicit pseudoinverse with a reusable grounded Laplacian
  factorization and solves. Recenter unpinned solutions so comparisons have the
  same translation convention. Verify against Julia on unpinned cases.
- [x] Treat pinned coordinates as constraints in the solve. This intentionally
  improves on solving freely and restoring pins afterward; test both pin
  preservation and constrained objective behavior, and document the difference.
- [x] Preserve the documented finite-distance convention between disconnected
  components, with a defined fallback when every node is isolated.
- [x] Test that stress does not increase beyond numerical tolerance for exact
  majorization updates, solve residuals are small, and the stopping conditions
  mean what they claim. Test coincident starting positions explicitly.
- [x] Reuse work buffers and the factorization across updates.
- [ ] Profile distance construction, factorization, and iteration separately.
  Deferred; current timings cover the full computation.

The initial exact method still needs quadratic pair storage and dense
factorization. Measure and document that ceiling. If it misses the agreed size
target, evaluate a residual-controlled iterative solve before introducing a
different sparse/pivot stress objective. Do not silently change the objective
or reduce iterations to claim a speedup.

Deliverable: tested stress layout in native Rust and WASM, with quality-matched
timings. This task is complete before lower-priority algorithms displace it.

## Task 4: Spring, followed by measured acceleration

- [x] Port the exact Fruchterman–Reingold forces, cooling schedule, dimensions,
  initialization, and pins. Compare fixed-initial-position iterates with Julia.
- [x] Compute attraction over edges and exact repulsion over pairs, reusing
  force buffers. Test singleton, disconnected, coincident, and pinned cases.
- [x] Add Barnes–Hut repulsion in 2D and 3D. Expose an accuracy parameter and
  retain exact evaluation for validation, small graphs, and higher dimensions.
- [x] Compare approximate forces with the exact kernel at several tolerances.
  Test self-force exclusion and coincident-point handling in the spatial tree.
- [x] Benchmark final layout quality as well as runtime. Select the automatic
  crossover from measurements; do not assume the tree wins for small inputs.

Deliverable: exact spring in configurable dimensions, accelerated common cases,
and a documented accuracy/performance tradeoff.

## Task 5: Spectral, shell, and Buchheim

- [x] Port Julia's spectral formulation: the generalized eigenproblem
  `L v = lambda D v`, including node weights. It differs from NetworkX's
  unnormalized Laplacian formulation.
- [x] Use a symmetric normalized formulation with a pure Rust solver. Prefer
  computing only the needed eigenpairs when supported and beneficial. Check
  residuals and convergence explicitly.
- [x] Define isolated-node and disconnected-component behavior and requests
  for more coordinates than a small graph can supply. Document any departure
  from Julia's singular or undersized cases.
- [x] Compare eigenvalues, residuals, and eigenspaces rather than raw signs or
  arbitrary bases in repeated-eigenvalue subspaces. Cover 2D, 3D, and at least
  one higher-dimensional case.
- [x] Port circular/shell placement, selected node groups, singleton inner
  shells, and the placement of omitted nodes. Reject repeated or invalid nodes.
- [x] Port Buchheim's subtree apportioning and variable node sizes. Precompute
  parents and sibling indices to eliminate Julia's repeated parent scans.
  Use explicit traversal stacks so deep trees do not overflow the WASM stack.
- [x] Test original tree cases plus singleton, very deep, wide, and invalid
  cyclic inputs. Assert sibling order, subtree separation, and parent centering.

Deliverable: all five selected layouts, with the relevant upstream tests ported
and additional numerical and boundary tests passing.

## Task 6: End-to-end performance and release checks

- [x] Measure native kernel time, CBOR overhead, complete fresh Typst compile
  time, and changed/unchanged incremental compilation separately.
- [ ] Complete the broad topology performance matrix. Current timings cover
  circulant graphs, sparse random graphs, and trees, including 10,000-node spring.
  Paths, wheels, meshes, and disconnected graphs have correctness tests but not
  a separate timing matrix. Stress/spectral have an explicit 4096-node cap.
- [x] Benchmark 2D and 3D; include a higher-dimensional correctness smoke test.
  Report warmups, repetitions, median and spread, CPU/toolchain, memory, WASM
  size, iteration counts, and convergence/quality measures.
- [x] Compare with Julia using equivalent initialization and stopping criteria.
  Compare Typst with diagraph-layout on shared inputs, clearly identifying
  differences in objectives, edge routing, and other work. Do not attribute the
  entire difference to the implementation language.
- [x] Add `make bench` and `make bench-typst`; keep timing thresholds out of
  heterogeneous CI. Use deterministic correctness and quality checks in CI.
- [x] Follow chalks' CI structure: formatting, Clippy, native tests, rebuilt WASM,
  Typst tests on 0.14.2 and 0.15.1, and reproducibility checks on the build platform.
- [x] Write a compiled manual and examples showing coordinates used by a
  renderer, without adding a rendering API to this package. Inspect representative
  figures so numerical tests do not conceal visually poor results.
- [x] Run a fresh-context implementation review using the review-implementation
  skill and address material findings before considering the implementation done.

Completion requires all selected algorithms and tests, a working bundled WASM,
documented deviations from Julia, and measured performance at stated quality.
Report missed speed targets honestly; the phrase "ultra fast" is not a substitute
for benchmark evidence.

## Current local state

All five selected algorithms, translated Julia fixtures, boundary tests, native
and CBOR APIs, and the bundled Typst WASM are implemented. Native and Typst tests
pass. A fresh reviewer found a coincident-free/pinned stress initialization bug;
it was fixed with a regression test for both node orders. Detailed results and
remaining measurement limits are in `docs/performance.md` and `docs/review.md`.

The private `GiggleLiu/fast-layout` repository has been created. The Typst
package has not been published to Universe. The unchecked items
above are additional profiling coverage, not missing layout algorithms.
