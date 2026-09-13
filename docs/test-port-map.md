# NetworkLayout.jl test port map

Reference: [NetworkLayout.jl 0.4.10 at commit
`073192ac737ff3309d7a1204fdd99ea361232d2b`](https://github.com/JuliaGraphs/NetworkLayout.jl/tree/073192ac737ff3309d7a1204fdd99ea361232d2b).
Julia uses one-based indices; Rust fixtures use zero-based indices.

The optional Julia reference environment fetches this exact commit. We retain
the generated JSON fixtures and the unchanged upstream
[`test/jagmesh1.mtx`](https://github.com/JuliaGraphs/NetworkLayout.jl/blob/073192ac737ff3309d7a1204fdd99ea361232d2b/test/jagmesh1.mtx)
in `fast-layout-engine/tests/fixtures/`. The matrix file's SHA-256 is
`88da6828587dba25012fb680885e1edf010e779636298151a26c253d7bc2372c`.
Upstream attribution and the full [MIT license](../fast-layout/NETWORKLAYOUT-LICENSE.md)
remain in the package. See [development instructions](development.md#benchmarks-and-reference-tests)
to regenerate the fixtures.

| Upstream test | Rust coverage | Status |
| --- | --- | --- |
| Stress iterator size | `upstream.rs`: fixture update count | Ported with corrected semantics: Rust counts force updates |
| Stress jagmesh 2D/3D | `upstream.rs`: original `jagmesh1.mtx` parses as 3,600 entries over 936 nodes and runs both dimensions | Ported as an ignored release-mode reference test because exact Stress uses dense solves |
| Stress wheel 2D/3D and wrapper equality | Weighted connected and disconnected 2D/3D fixtures | Replaced by fixed-input numerical comparison |
| Stress pairwise distance | `graph.rs`: cycle and weighted-shortest-path assertions; fixture objectives use the same distances | Ported |
| Stress disconnected graphs | Weighted disconnected 2D/3D fixtures | Ported |
| Spring iterator size | `upstream.rs`: fixture update count | Ported with corrected semantics |
| Spring wheel 2D/3D and wrapper equality | Exact-force 2D/3D fixtures with `theta = 0` | Replaced by fixed-input numerical comparison |
| Spring singleton | `upstream.rs` singleton request | Ported |
| Spectral wheel 3D and wrapper equality | Weighted 2D/3D/5D eigenvalue, residual, distance, and subspace invariants | Replaced because eigenvector signs and repeated-eigenvalue bases are not stable |
| Shell wheel and singleton | `shell_geometry.json` plus singleton request | Ported; fixture also covers explicit shells, singleton inner shell, and omitted nodes |
| Buchheim matrix conversion | Request edge conversion and validation tests | Ported to the indexed-edge API |
| Buchheim original varying-size tree | `buchheim_varying_size.json` | Ported |
| Buchheim binary tree | Tree geometry assertions | Ported |
| Buchheim invalid tree requirements | Multiple-parent, root, cycle, and unreachable tests | Ported and expanded |
| Graphs extension glue | Indexed edge-list construction tests | Replaced by the Rust-native graph input contract |
| Square-adjacency assertion | Request node count and edge-bound validation | Replaced by the indexed-edge API, which has no adjacency shape |
| Symmetric adjacency conversion | Graph duplicate/reverse-edge normalization tests | Ported to undirected edge normalization; conflicting duplicate weights are rejected |
| Initial position and pin sanitization | Request validation and per-coordinate pin tests | Ported to typed Rust input |
| Stress and Spring pin behavior | Pin preservation and constrained stress tests | Ported; stress deliberately solves constraints instead of Julia's post-solve clamp |
| Generic callable layout macro and manual layout | None | Excluded: Julia dispatch/macro API |
| Float32, integer, inferred point constructors | None | Excluded: Rust API uses `f64` and explicit dimensions |
| Repeated iterator determinism and iterator API | Deterministic request tests | Iterator API excluded; Rust returns one final response |
| SFDP tests | None | Excluded: algorithm is outside the selected port |
| Square-grid tests | None | Excluded: algorithm is outside the selected port |
| Alignment tests | None | Excluded: feature is outside the selected port |

Julia's `LayoutIterator` emits the initial coordinates as its first item. Its
Spring and Stress state stop when `iteration >= iterations`, so an iterator
configured with `n` items performs `n - 1` numerical updates. The public
`layout()` loop has a separate off-by-one bug: when the iterator ends, it keeps
the preceding item. For `n >= 2`, it therefore returns the state after `n - 2`
updates. `scripts/reference/audit_iterations.jl` executes this distinction.
The fixture generator bypasses `layout()`, constructs the iterator with
`requested_updates + 1`, and explicitly advances it for the requested number
of updates. Rust's `Response.iterations` counts completed numerical updates.

Julia's Spectral layout adds both adjacency triangles, so an already symmetric
matrix doubles every affinity. The fixture passes half-weights to Julia and the
declared undirected weights to Rust. This preserves the same normalized
generalized eigenproblem without giving duplicate edges a second meaning.
