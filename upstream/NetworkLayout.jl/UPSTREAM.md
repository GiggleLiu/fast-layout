# Upstream snapshot

- Project: NetworkLayout.jl
- Source: https://github.com/JuliaGraphs/NetworkLayout.jl
- Version in `Project.toml`: 0.4.10
- Commit: `073192ac737ff3309d7a1204fdd99ea361232d2b`
- License: MIT Expat, reproduced in `LICENSE.md`

The source, tests, and `jagmesh1.mtx` are copied without changes. The
`ext/NetworkLayoutGraphsExt.jl` file completes the runnable snapshot and is
also copied verbatim from the pinned commit.

The `v0.4.10` Git tag currently resolves to
`a30a29ace70d6ee6de8f41c41cd5777e5bf19f37`. This repository follows the
explicit commit requested for the port, whose `Project.toml` still reports
version 0.4.10.

Verify every copied file against `SNAPSHOT.toml` with:

```sh
julia scripts/reference/verify_snapshot.jl
```
