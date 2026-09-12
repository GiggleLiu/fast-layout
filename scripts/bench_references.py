#!/usr/bin/env python3
"""Benchmark 100-node reference graph-layout implementations."""

import argparse
from datetime import date
import hashlib
import json
import math
import os
import platform
import random
import statistics
import subprocess
import sys
import tempfile
import time
from collections import deque
from importlib.metadata import version
from pathlib import Path

os.environ.setdefault("OMP_NUM_THREADS", "1")

import igraph as ig
import s_gd2

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/benchmarks/references-100.json"
RUST_OUTPUT = ROOT / "docs/benchmarks/rust-reference-100.json"
GRAPH_DIR = ROOT / "_bench/reference-graphs"
N = 100
REPEATS = 5
SEED = 1


def graphs():
    path = [(i - 1, i) for i in range(1, N)]
    cycle = path + [(N - 1, 0)]
    grid = [(r * 10 + c, r * 10 + c + 1) for r in range(10) for c in range(9)]
    grid += [(r * 10 + c, (r + 1) * 10 + c) for r in range(9) for c in range(10)]
    random_edges = list(path)
    rng = random.Random(SEED)
    seen = set(path)
    for i in range(N):
        edge = tuple(sorted((i, rng.randrange(N))))
        if edge[0] != edge[1] and edge not in seen:
            seen.add(edge)
            random_edges.append(edge)
    return {"path": path, "cycle": cycle, "grid-10x10": grid, "connected-random": random_edges}


def distances(edges):
    adjacent = [[] for _ in range(N)]
    for a, b in edges:
        adjacent[a].append(b)
        adjacent[b].append(a)
    result = []
    for source in range(N - 1):
        row = [-1] * N
        row[source] = 0
        queue = deque([source])
        while queue:
            node = queue.popleft()
            for neighbor in adjacent[node]:
                if row[neighbor] < 0:
                    row[neighbor] = row[node] + 1
                    queue.append(neighbor)
        result.extend((source, target, row[target]) for target in range(source + 1, N))
    assert len(result) == N * (N - 1) // 2 and all(d > 0 for _, _, d in result)
    return result


def stress(positions, pairs):
    total = 0.0
    for i, j, graph_distance in pairs:
        x = positions[i][0] - positions[j][0]
        y = positions[i][1] - positions[j][1]
        total += ((math.hypot(x, y) - graph_distance) / graph_distance) ** 2
    return {"raw": total, "per_pair": total / len(pairs), "pairs": len(pairs)}


def timed(call):
    call()  # warm-up
    samples = []
    result = None
    for _ in range(REPEATS):
        started = time.perf_counter_ns()
        result = call()
        samples.append((time.perf_counter_ns() - started) / 1_000_000)
    return result, {"samples_ms": samples, "median_ms": statistics.median(samples)}


def cpu_name():
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return platform.processor()


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rust_source_sha256():
    digest = hashlib.sha256()
    for path in sorted((ROOT / "fast-layout-engine/src").glob("*.rs")):
        digest.update(path.name.encode())
        digest.update(path.read_bytes())
    return digest.hexdigest()


def benchmark_rust(reference):
    results = []
    with tempfile.TemporaryDirectory(prefix="fast-layout-rust-reference-") as directory:
        for graph_name in graphs():
            for stress_method, iterations in (("sgd", 15), ("majorization", 100)):
                output = Path(directory) / f"{graph_name}-{stress_method}.json"
                subprocess.run(
                    [
                        "cargo", "bench", "-q", "-p", "fast-layout-engine", "--bench", "layouts", "--",
                        "--algorithms", "stress", "--sizes", str(N), "--dim", "2",
                        "--iterations", str(iterations), "--repeats", str(REPEATS), "--tolerance", "0",
                        "--stress-method", stress_method, "--graph-file", str(GRAPH_DIR / f"{graph_name}.json"),
                        "--format", "json", "--output", str(output),
                    ],
                    cwd=ROOT,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                reference_method = "s_gd2-15-0.1" if stress_method == "sgd" else "s_gd2-30-0.01"
                baseline = reference["graphs"][graph_name]["methods"][reference_method]
                for row in json.loads(output.read_text()):
                    row["graph"] = graph_name
                    row["reference"] = {
                        "method": reference_method,
                        "median_ms": baseline["timing"]["median_ms"],
                        "objective": baseline["stress"]["raw"],
                        "objective_ratio": row["objective"] / baseline["stress"]["raw"],
                        "settings_match": stress_method == "sgd",
                    }
                    results.append(row)
    plugin = ROOT / "fast-layout/plugin/fast_layout_engine.wasm"
    report = {
        "metadata": {
            "created": date.today().isoformat(),
            "machine": reference["metadata"]["machine"],
            "benchmark": "cargo bench -p fast-layout-engine --bench layouts (release profile)",
            "method": "one untimed warm-up, then five serial wall-clock samples per case",
            "reference_file": str(OUTPUT.relative_to(ROOT)),
            "rust_source_sha256": rust_source_sha256(),
            "plugin_wasm_sha256": sha256(plugin),
            "note": "s_gd2 timings include its Python wrapper. Rust compute and CBOR boundary timings are separate. SGD uses the paper's 15-epoch epsilon=0.1 schedule; majorization has a different optimizer and is compared only by the common exact stress objective.",
        },
        "settings": {
            "nodes": N, "dim": 2, "tolerance": 0.0, "repeats": REPEATS,
            "initial": "(sin(i), cos(i)) for one-based i",
        },
        "results": results,
    }
    RUST_OUTPUT.write_text(json.dumps(report, indent=2) + "\n")
    print(RUST_OUTPUT)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-rust", action="store_true", help="run only Python reference implementations")
    args = parser.parse_args()
    initial = [[math.sin(i), math.cos(i)] for i in range(1, N + 1)]
    report = {
        "metadata": {
            "created": date.today().isoformat(),
            "machine": {"cpu": cpu_name(), "platform": platform.platform(), "python": platform.python_version()},
            "method": "one untimed warm-up, then five serial wall-clock samples per case",
            "environment": sys.prefix,
            "packages": {"s_gd2": version("s_gd2"), "python-igraph": version("python-igraph"), "igraph": ig.__version__},
            "sources": {
                "s_gd2": {
                    "url": "https://github.com/jxz12/s_gd2",
                    "license": "MIT",
                    "license_checked_at_revision": "52ab0a5bee183b45eed60061cabe8e63f006e8a0",
                },
                "algorithm_paper": {
                    "title": "Graph Drawing by Stochastic Gradient Descent",
                    "url": "https://arxiv.org/abs/1710.04626",
                    "authors": ["Jonathan X. Zheng", "Samraat Pawar", "Dan F. M. Goodman"],
                    "schedule": "Section II-A1 selects t_max=15 and epsilon=0.1 as a speed-quality compromise",
                },
                "python-igraph": {"url": "https://igraph.org/python/", "license": "GPL-2.0-or-later"},
            },
            "stress": "sum over unordered pairs of ((Euclidean distance - unweighted shortest-path distance) / shortest-path distance)^2; per_pair divides by 4950; coordinates are not rescaled",
            "comparability": "s_gd2 optimizes this stress objective. igraph FR and KK timings are separate layout objectives, so their diagnostic stress values are not quality comparisons.",
        },
        "settings": {"nodes": N, "seed": SEED, "repeats": REPEATS, "initial": initial},
        "graphs": {},
    }
    GRAPH_DIR.mkdir(parents=True, exist_ok=True)
    for name, edges in graphs().items():
        (GRAPH_DIR / f"{name}.json").write_text(json.dumps(edges) + "\n")
        pairs = distances(edges)
        I, J = zip(*edges)
        graph = ig.Graph(n=N, edges=edges, directed=False)
        methods = {}
        for t_max, eps in ((15, 0.1), (30, 0.01)):
            positions, timing = timed(lambda: s_gd2.layout(I, J, t_max=t_max, eps=eps, random_seed=SEED, init=initial))
            methods[f"s_gd2-{t_max}-{eps}"] = {
                "parameters": {"t_max": t_max, "eps": eps, "random_seed": SEED},
                "pair_updates": t_max * len(pairs),
                "timing": timing,
                "stress": stress(positions, pairs),
                "positions": positions.tolist(),
            }
        positions, timing = timed(lambda: graph.layout_fruchterman_reingold(niter=100, seed=initial).coords)
        methods["igraph-fr-100"] = {
            "parameters": {"niter": 100, "initial": "shared"},
            "timing": timing,
            "diagnostic_stress_not_comparable": stress(positions, pairs),
            "positions": positions,
        }
        positions, timing = timed(lambda: graph.layout_kamada_kawai().coords)
        methods["igraph-kk-default"] = {
            "parameters": {"defaults": True},
            "timing": timing,
            "diagnostic_stress_not_comparable": stress(positions, pairs),
            "positions": positions,
        }
        report["graphs"][name] = {"edges": edges, "methods": methods}
        print(name, ", ".join(f"{method}: {data['timing']['median_ms']:.3f} ms" for method, data in methods.items()))
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n")
    print(OUTPUT)
    if not args.skip_rust:
        benchmark_rust(report)


if __name__ == "__main__":
    main()
