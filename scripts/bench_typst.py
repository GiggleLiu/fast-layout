#!/usr/bin/env python3
"""Measure fresh Typst processes for fast-layout on deterministic graphs."""

import argparse
import hashlib
import json
import platform
import random
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "_bench"
DOCS = OUT / "docs"
VALID = {"stress", "spring", "spectral", "shell", "buchheim"}


def csv(value, cast=str):
    return [cast(item) for item in value.split(",") if item]


def graph(n):
    """A connected sparse graph, stable across Python versions."""
    edges = [(i - 1, i) for i in range(1, n)]
    rng = random.Random(1)
    seen = set(edges)
    for i in range(n):
        edge = tuple(sorted((i, rng.randrange(n))))
        if edge[0] != edge[1] and edge not in seen:
            seen.add(edge)
            edges.append(edge)
    return edges


def typst_array(items):
    if not items:
        return "()"
    return "(" + ",".join(f"({a},{b})" for a, b in items) + ",)"


def ours_source(n, edges, algorithm):
    return (
        '#import "@preview/fast-layout:0.1.0": layout\n'
        f"#let result = layout({n}, {typst_array(edges)}, algorithm: \"{algorithm}\")\n"
        "#assert.eq(result.positions.len(), " + str(n) + ")\n"
    )


def compare_source(n, edges, engine):
    elements = [f'node("{i}")' for i in range(n)]
    elements += [f'edge("{a}", "{b}")' for a, b in edges]
    return (
        '#import "@preview/diagraph-layout:0.0.1": layout-graph, node, edge\n'
        f'#let result = layout-graph(engine: "{engine}", {",".join(elements)})\n'
        "#assert(not result.errored)\n"
        f"#assert.eq(result.nodes.len(), {n})\n"
    )


def render_source(n, edges, algorithm):
    return (
        '#import "@preview/fast-layout:0.1.0": layout\n'
        f"#let edges = {typst_array(edges)}\n"
        f'#let result = layout({n}, edges, algorithm: "{algorithm}")\n'
        "#let xs = result.positions.map(p => p.at(0))\n"
        "#let ys = result.positions.map(p => p.at(1))\n"
        "#let min-x = calc.min(..xs)\n"
        "#let min-y = calc.min(..ys)\n"
        "#let span-x = calc.max(calc.max(..xs) - min-x, 0.001)\n"
        "#let span-y = calc.max(calc.max(..ys) - min-y, 0.001)\n"
        "#let points = result.positions.map(p => ((5 + 190 * (p.at(0) - min-x) / span-x) * 1pt, (5 + 190 * (p.at(1) - min-y) / span-y) * 1pt))\n"
        "#box(width: 200pt, height: 200pt)[\n"
        "  #for edge in edges { place(line(start: points.at(edge.at(0)), end: points.at(edge.at(1)), stroke: 0.4pt)) }\n"
        "  #for point in points { place(dx: point.at(0), dy: point.at(1), circle(radius: 1.5pt, fill: black)) }\n"
        "]\n"
    )


def diagraph_render_source(n, edges, engine):
    statements = [f'{i} [label=""]' for i in range(n)]
    statements += [f"{a} -- {b}" for a, b in edges]
    dot = "graph { node [shape=circle width=0.05 height=0.05 fixedsize=true]; " + ";".join(statements) + " }"
    return (
        '#import "@preview/diagraph:0.3.7": render\n'
        f'#render({json.dumps(dot)}, engine: "{engine}", width: 200pt, height: 200pt)\n'
    )


def run(doc, timeout):
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            ["typst", "compile", "--root", str(ROOT), str(doc), str(OUT / "out.pdf")],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "seconds": time.perf_counter() - started,
            "status": "ok" if proc.returncode == 0 else "error",
            "returncode": proc.returncode,
            "stderr": proc.stderr[-4000:],
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "seconds": time.perf_counter() - started,
            "status": "timeout",
            "returncode": None,
            "stderr": (exc.stderr or "")[-4000:] if isinstance(exc.stderr, str) else "",
        }


def write_case(name, source):
    path = DOCS / f"{name}.typ"
    path.write_text(source)
    return path


def machine_info():
    cpu = platform.processor()
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass
    version = subprocess.run(["typst", "--version"], capture_output=True, text=True, check=True)
    return {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "cpu": cpu,
        "python": platform.python_version(),
        "typst": version.stdout.strip(),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", default="100,500,1000", help="comma-separated connected-graph sizes")
    parser.add_argument("--algorithms", default="stress,spring,spectral,shell,buchheim")
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--compare", action="store_true", help="also run diagraph-layout neato and sfdp")
    parser.add_argument("--compare-render", action="store_true", help="render with fast-layout and diagraph neato/sfdp")
    parser.add_argument("--output", default="_bench/results.json")
    args = parser.parse_args()
    sizes = csv(args.sizes, int)
    algorithms = csv(args.algorithms)
    unknown = set(algorithms) - VALID
    if unknown or not sizes or min(sizes) < 1 or args.repeats < 1 or args.timeout <= 0:
        parser.error("use known algorithms, positive sizes/repeats, and a positive timeout")

    DOCS.mkdir(parents=True, exist_ok=True)
    baseline = write_case("baseline", "= fast-layout benchmark\n")
    cases = [("baseline", baseline)]
    for n in sizes:
        edges = graph(n)
        for algorithm in algorithms:
            if algorithm in {"stress", "spring", "spectral"}:
                cases.append((f"{algorithm}-{n}", write_case(f"{algorithm}-{n}", ours_source(n, edges, algorithm))))
        if args.compare:
            for engine in ("neato", "sfdp"):
                name = f"diagraph-{engine}-{n}"
                cases.append((name, write_case(name, compare_source(n, edges, engine))))
        if args.compare_render:
            for algorithm in ("stress", "spring"):
                name = f"fast-layout-render-{algorithm}-{n}"
                cases.append((name, write_case(name, render_source(n, edges, algorithm))))
            for engine in ("neato", "sfdp"):
                name = f"diagraph-render-{engine}-{n}"
                cases.append((name, write_case(name, diagraph_render_source(n, edges, engine))))
    if "shell" in algorithms:
        cases.append(("shell-10000", write_case("shell-10000", ours_source(10_000, [], "shell"))))
    if "buchheim" in algorithms:
        tree = [((i - 1) // 2, i) for i in range(1, 10_000)]
        cases.append(("buchheim-10000", write_case("buchheim-10000", ours_source(10_000, tree, "buchheim"))))

    run(baseline, args.timeout)  # Warm the compiler before timing.
    if args.compare:
        for name, doc in cases:
            if name.startswith("diagraph-"):
                run(doc, args.timeout)  # Download the optional package outside timed runs.
                break
    if args.compare_render:
        warm = write_case("diagraph-render-warm", diagraph_render_source(1, [], "neato"))
        run(warm, args.timeout)  # Download and initialize diagraph outside timed runs.
    output = ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "machine": machine_info(),
        "wasm_sha256": hashlib.sha256(
            (ROOT / "fast-layout/plugin/fast_layout_engine.wasm").read_bytes()
        ).hexdigest(),
        "settings": vars(args),
        "results": [],
    }
    for name, doc in cases:
        samples = [run(doc, args.timeout) for _ in range(args.repeats)]
        report["results"].append({"case": name, "document": str(doc.relative_to(ROOT)), "samples": samples})
        output.write_text(json.dumps(report, indent=2) + "\n")
        print(f"{name}: " + ", ".join(f'{s["status"]} {s["seconds"]:.3f}s' for s in samples))
    print(f"raw results: {output}")


if __name__ == "__main__":
    main()
