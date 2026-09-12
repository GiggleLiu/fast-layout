# Implementation review

A reviewer with no implementation history read all production Rust modules,
the Typst bridge, tests, build configuration, and benchmark scripts on 2026-09-12.
Mechanical porting, test expansion, and benchmark collection used GPT-5.6-Sol.

The reviewer found one numerical bug. Stress initialization perturbed only later
members of a coincident group. If the later node was pinned and the earlier node
was free, neither moved and the solver incorrectly stopped at positive stress.
The fix groups the original coordinates and perturbs every movable member of a
duplicate group. A regression test covers both node orders and verifies exact
pins and zero final stress on a two-node edge. The reviewer reran the public API
reproducer and confirmed the fix.

The reviewer found no further material production-code issues in constrained
stress solves, sparse spectral residual checks, Barnes–Hut self-force exclusion,
or iterative Buchheim traversal. The review also requested duplicate/reverse-edge
and invalid-root tests, corrected reference links and plan status, and final
Barnes–Hut layout quality measurements. Those checks and records were added.

## Validation

- Rust formatting and Clippy with warnings denied pass.
- Native unit tests, API tests, and independent Julia numerical fixtures pass,
  38 tests in the default run plus the separate jagmesh test.
- The ignored 936-node jagmesh test is run separately in release mode.
- Typst 0.14.2 and 0.15.1 package tests exercise the bundled WASM; example figures were compiled
  and visually inspected.
- The Typst package checker passes. A clean-target WASM rebuild is byte-identical.
- `make install` resolves the package in the local Typst preview namespace.

Final command results, runtime measurements, and measurement limits are recorded
in [performance.md](performance.md). This document does not claim a remote CI run.

## Git audit availability

The referenced `github-pr-audit` skill was unavailable in both skill directories
and this checkout. At the time of that initial review, this directory had no Git repository or
GitHub PR, so remote metadata and an official audit gate were unavailable. The reviewer inspected
candidate artifacts read-only and found the expected bundled WASM, license files,
manuals, and ignored build/cache outputs, with no material artifact concern.

## Rename and private repository

The package, crate, WASM, imports, examples, build commands, and local installation
were renamed to `fast-layout`. The existing correctness tests passed after the
rename. Historical benchmark hashes retain their original values; the new
rendered-diagram comparison records the renamed build separately.

The user requested a private repository under GiggleLiu. GitHub metadata confirms
`GiggleLiu/fast-layout` is private. Generated target directories, package symlinks,
benchmark scratch files, and Python caches are excluded from Git. Bundled WASM,
compiled manuals linked from the README, source fixtures, licenses, and benchmark
records are intentional repository artifacts.

A second fresh review checked the rename and rendered benchmark extension. It
found no material implementation issue and requested verification that diagraph
had not drawn an error block while returning a successful compiler status. Every
successful engine/size case was recompiled and its PDF text checked for errors.
The README table reports all timeouts and documents differing rendering work.

The static site is one self-contained HTML file. Browser checks covered all five
selectors, keyboard navigation, selected-tab state, and desktop/mobile layouts.
The circular example uses uniform coordinate scaling so circles stay circular.
NetworkLayout.jl attribution appears in the website and both READMEs.

## 100-node speed and CeTZ revision

A fresh reviewer checked the new SGD solver, dimension specializations, exact
2D spring kernel, CBOR API, tests, benchmark scripts and records, and manuals.
No material numerical defect was found. The review confirmed source and WASM
hashes, native comparisons, and warm timings that exclude the first call.

The profiler now decodes timeout diagnostics before writing JSON. A focused
reproducer verified that non-UTF-8 diagnostics remain serializable. Documentation
now distinguishes majorization's Cholesky setup from SGD pair updates, explains
SGD's stopping rule, and scopes the zero-update fully pinned case to stress.

Validation passed for 43 native tests plus the separate release jagmesh case,
Rust formatting, Clippy with warnings denied, Typst numerical/error tests on
0.14.2 and 0.15.1, and the Typst package checker. All CeTZ documents compiled on
both Typst versions. The four-page manual and gallery were visually inspected;
the manual's complete CeTZ snippet was also extracted and compiled independently.
A clean-target rebuild produced the identical 285,138-byte WASM recorded in
[the 100-node report](performance-100.md).

The Git audit skill remains unavailable, so there is no delegated audit gate
result. Read-only artifact inspection found the expected WASM, compiled manuals,
and benchmark records. GitHub metadata confirms the repository remains private.

## SGD default

A fresh review found no material issue in the SGD default change. Optional
iteration limits resolve to 15 for stress SGD and 100 for other methods;
explicit limits remain intact and zero is rejected. Native and CBOR requests
share this resolution. Rust callers now use `Some(limit)` for overrides.
Majorization fixtures select their solver explicitly; fixture data is unchanged.

All 44 native tests, the release jagmesh case, Clippy, both supported Typst
versions, README snippets, and the package checker pass. The regenerated manual
remains four pages and was visually inspected. The current profile records 135
successful samples and the README uses its medians. Review confirmed the final
documentation, defaults, and recorded WASM hash agree.
