# Julia reference baseline

Measured 2026-09-12 with Julia 1.12.6, one Julia thread, and one OpenBLAS
thread on an Intel Xeon Gold 6226R at 2.90 GHz. Each result is the median of
three warm runs after one warmup.

The input is a circulant graph with offsets 1, 7, and 31. It has `n` vertices
and `3n` undirected edges. The 2D initial position for one-based vertex `i` is
`(sin(i), cos(i))`; 3D adds `sin(2i)`. Tolerances are zero. Counts below are
completed numerical updates, so the Julia constructors receive one extra
iteration to account for their initial iterator item. The benchmark consumes
`LayoutIterator` directly. It does not call Julia's buggy public `layout()`
helper, which returns its penultimate iterator item and loses one more update.

| algorithm | nodes | dim | updates | median seconds |
| --- | ---: | ---: | ---: | ---: |
| spring | 100 | 2 | 20 | 0.002933 |
| stress | 100 | 2 | 5 | 0.002972 |
| spring | 100 | 3 | 20 | 0.003121 |
| stress | 100 | 3 | 5 | 0.003598 |
| spring | 500 | 2 | 20 | 0.067345 |
| stress | 500 | 2 | 5 | 0.228182 |
| spring | 500 | 3 | 20 | 0.071331 |
| stress | 500 | 3 | 5 | 0.223015 |
| spring | 1000 | 2 | 20 | 0.264172 |
| stress | 1000 | 2 | 5 | 4.876980 |
| spring | 1000 | 3 | 20 | 0.281755 |
| stress | 1000 | 3 | 5 | 4.806958 |

The initial Rust target is no slower than these medians on this machine for
the same requests. Quality must also match the fixed Julia fixtures: Spring
coordinates within `2e-10`, unpinned Stress pairwise distances and objective
within `2e-8`, and Spectral generalized-eigen residuals below `2e-9`.

Run the benchmark with:

```sh
OPENBLAS_NUM_THREADS=1 julia --project=scripts/reference scripts/reference/benchmark.jl
```

Exact Stress forms dense pair matrices and computes a pseudoinverse in Julia.
The 1,000-node timing shows that cost; it is a measured ceiling, not a target
for graphs that exceed available memory.
