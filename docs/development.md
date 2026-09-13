# Development and release preparation

The Rust toolchain and WASM target are pinned in `rust-toolchain.toml`; Cargo
dependencies are pinned in `Cargo.lock`. Install Rust through rustup and Typst
0.14.2 or newer. Julia and Python reference libraries are optional and are not
needed to build or use the package.

```sh
make test           # Rust tests, WASM rebuild, Typst tests, and CeTZ manual
make check-package  # Stage the submission, check it, compile README examples
```

`make check-package` additionally requires Python 3.11+ and Docker. CI runs both
commands with Typst 0.14.2 and 0.15.1 and verifies that the rebuilt WASM matches
the committed binary. `make plugin` rebuilds only the WASM; `make manual`
regenerates the single PDF manual.

## Submission files

`make package` stages `_dist/preview/fast-layout/0.1.0/`. This contains the
manifest, public Typst files, WASM, README, licenses, and the linked PDF manual.
Tests, build scripts, reference scripts, benchmark records, and
the introduction website stay in the development repository. The manifest
excludes the manual files from the compiler's download bundle while keeping
them available on Typst Universe.

Before submitting, run `make test` and `make check-package`. Copy the staged
directory to `packages/preview/fast-layout/0.1.0/` in a fork of `typst/packages`
and open the package submission PR following the official
[submission guidelines](https://github.com/typst/packages/blob/main/docs/README.md).
Staging files does not publish them. This development repository is currently
private; a submission to `typst/packages` publishes the submitted files. The
optional `repository` manifest field is omitted because the package checker
rejects inaccessible URLs. Add it when there is a public source repository.

## Benchmarks and reference tests

`make bench-typst` runs the [warm package comparison](warm-comparison.md).
`make bench` measures native kernels. `scripts/profile_typst.py` provides a
more detailed iteration/startup profile and writes scratch results under `_bench`.
The [native reference comparison](performance-100.md) documents optional C++
benchmarks. Historical experiments remain in Git history.

The [test port map](test-port-map.md) records Julia provenance and numerical
fixtures. To regenerate them, use Julia 1.12 and fetch the pinned NetworkLayout.jl
commit into Julia's package cache:

```sh
julia --project=scripts/reference -e 'using Pkg; Pkg.instantiate()'
OPENBLAS_NUM_THREADS=1 julia --project=scripts/reference scripts/reference/generate.jl
```

The first command requires network access. Ordinary Rust tests use the committed
fixtures and need no Julia installation or upstream checkout.
The larger upstream reference case is separate from the routine test suite:

```sh
cargo test --locked --release --test upstream -- --ignored
```
