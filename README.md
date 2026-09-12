# fast-layout

Pure Rust stress, spring, spectral, circular/shell, and Buchheim tree layouts,
with a bundled WASM plugin for Typst. Numerical layouts accept configurable
dimensions, including 2D, 3D, and higher dimensions.

The algorithms and behavioral tests are adapted from
[NetworkLayout.jl](https://github.com/JuliaGraphs/NetworkLayout.jl) 0.4.10,
created by Abhijith Anilkumar and other contributors. Its [MIT license](fast-layout/NETWORKLAYOUT-LICENSE.md)
and [source attribution](THIRD_PARTY-NOTICES.md) are retained.

See [the API documentation](fast-layout/README.md) for Typst examples, options,
weights, pins, convergence, and size limits. The package is local, not yet
published to Typst Universe.

```rust
use fast_layout_engine::{compute, Algorithm, Request};
let mut request = Request::new(4, vec![[0, 1], [1, 2], [2, 3]], Algorithm::Stress);
request.dim = 3;
let result = compute(&request)?;
```

## Benchmarks

Complete rendered Typst documents, median of three fresh compilations. Each
method receives the same connected graph with about two edges per node. Lower
is faster. Measured on an Intel Xeon Gold 6226R, Typst 0.15.1, against
[diagraph 0.3.7](https://typst.app/universe/package/diagraph/).

| Nodes | fast-layout Stress | fast-layout Spring | diagraph `neato` | diagraph `sfdp` |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 0.223 s | 0.199 s | 1.627 s | 0.832 s |
| 500 | 5.202 s | 1.900 s | >20 s timeout | 5.408 s |
| 1,000 | >20 s timeout | 4.264 s | >20 s timeout | 15.927 s |

These are different algorithms and rendering paths. fast-layout draws straight
edges with native Typst; diagraph uses Graphviz routing and rendering. fast-layout
uses its default 100-update limit and tolerance. These timings do not establish
equal layout quality. A timeout means the run exceeded the 20-second test limit.

[Methodology and raw results](docs/diagraph-comparison.md) include coordinate-only
controls and the exact WASM hash. [Native Rust/Julia timings](docs/performance.md)
are recorded separately.

## Layout guide

Open [site/index.html](site/index.html) in a browser for visual explanations of
the five layouts. It is a self-contained static page with no build step.

## Development

```sh
make test         # native tests, WASM build, Typst tests, and manual
make examples     # compile example figures
make bench        # native and CBOR benchmarks
make bench-typst  # fresh Typst process benchmarks
make plugin       # reproducibly rebuild the bundled WASM
make install      # install into the local Typst preview namespace
```

Rust 1.93.1 and `wasm32-unknown-unknown` are pinned in `rust-toolchain.toml`.
The runtime uses nalgebra's pure Rust routines and has no BLAS, LAPACK, Graphviz,
Python, or Julia dependency. The layout API returns coordinates; drawing and
edge routing belong to the caller.

[Performance measurements](docs/performance.md) distinguish native computation,
CBOR overhead, fresh Typst compilation, and incremental compilation.
[The diagraph comparison](docs/diagraph-comparison.md) adds full rendered-document
timings against diagraph 0.3.7.

[UPSTREAM.md](upstream/NetworkLayout.jl/UPSTREAM.md) pins NetworkLayout.jl 0.4.10 source and test provenance.
[The test map](docs/test-port-map.md) records translated cases and exclusions.
Run `julia --project=scripts/reference scripts/reference/verify_snapshot.jl` to check the snapshot, and
`cargo test --release --test upstream -- --ignored` for the 936-node jagmesh case.
See [third-party notices](THIRD_PARTY-NOTICES.md) for copied and adapted code licenses.
