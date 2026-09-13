#!/usr/bin/env python3
"""Compare warm, uncached fast-layout and diagraph-layout calls in Typst."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

from bench_typst import ROOT, graph, machine_info, typst_array
from profile_typst import run_case


def cases(calls):
    edges = graph(100)
    result = {}
    for name, options in (
        ("stress-sgd", 'algorithm: "stress"'),
        ("stress-majorization", 'algorithm: "stress", stress-method: "majorization"'),
        ("spring", 'algorithm: "spring"'),
        ("spectral", 'algorithm: "spectral"'),
        ("shell", 'algorithm: "shell"'),
    ):
        source = (
            '#import "@preview/fast-layout:0.1.0": layout\n'
            f"#let edges = {typst_array(edges)}\n"
        )
        for seed in range(1, calls + 1):
            source += (
                f"#let result = layout(100, edges, {options}, seed: {seed})\n"
                "#assert.eq(result.positions.len(), 100)\n"
            )
        result[f"fast-layout-{name}"] = source
    elements = [f'node("{i}")' for i in range(100)]
    elements += [f'edge("{a}", "{b}")' for a, b in edges]
    for engine in ("neato", "fdp", "sfdp"):
        source = (
            '#import "@preview/diagraph-layout:0.0.1": layout-graph, node, edge\n'
            f"#let elements = ({','.join(elements)},)\n"
        )
        for seed in range(1, calls + 1):
            source += (
                f'#let result = layout-graph(engine: "{engine}", '
                f'start: "{seed}", ..elements)\n'
                "#assert(not result.errored)\n"
                "#assert.eq(result.nodes.len(), 100)\n"
                f"#assert.eq(result.edges.len(), {len(edges)})\n"
            )
        result[f"diagraph-layout-{engine}"] = source
    return result


def api_durations(events):
    """Outermost function spans containing one plugin export call per layout.

    Documents call each public API directly, without a surrounding loop/closure.
    Track all nested function spans so serialization stays inside the API span.
    """
    stacks = {}
    durations = []
    for event in events:
        key = (event.get("pid"), event.get("tid"))
        stack = stacks.setdefault(key, [])
        if event.get("name") == "func call":
            if event.get("ph") == "B":
                stack.append({"start": event["ts"], "plugins": 0})
            elif event.get("ph") == "E":
                span = stack.pop()
                if not stack and span["plugins"]:
                    if span["plugins"] != 1:
                        raise ValueError("expected exactly one plugin call per API call")
                    durations.append((event["ts"] - span["start"]) / 1000)
        elif event.get("name") == "call plugin" and event.get("ph") == "B":
            if not stack:
                raise ValueError("plugin call has no enclosing API span")
            stack[0]["plugins"] += 1
    if any(stacks.values()):
        raise ValueError("unclosed function spans")
    return durations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--calls", type=int, default=5)
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--output", default="docs/benchmarks/warm-comparison-100.json")
    parser.add_argument(
        "--diagraph-package", type=Path,
        default=Path.home() / ".cache/typst/packages/preview/diagraph-layout/0.0.1",
        help="installed diagraph-layout 0.0.1 directory, used to record its WASM hash",
    )
    args = parser.parse_args()
    if args.repeats < 1 or args.calls < 2 or args.timeout <= 0:
        parser.error("repeats >= 1, calls >= 2 and timeout > 0 are required")
    sources = cases(args.calls)
    report = {
        "recorded_at": datetime.now(timezone.utc).isoformat(),
        "machine": machine_info(),
        "git_revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "settings": {**vars(args), "diagraph_package": str(args.diagraph_package)},
        "graph": {"nodes": 100, "edges": graph(100), "directed": False},
        "method": {
            "samples": "fresh processes, cases run serially round-robin",
            "warm": "discard first API and plugin call in each process",
            "cache": "distinct seeds 1..calls: fast-layout seed; Graphviz start",
            "api": "outermost function span containing one plugin call, includes encoding and decoding",
            "plugin": "call plugin trace span, excludes outer Typst encoding and decoding",
            "output": "blank one-page PDF; assert 100 positions and no Graphviz layout error",
            "scope": "fast-layout returns positions; Graphviz also handles node sizes and edge routes; budgets and quality are not matched",
        },
        "results": {name: {"source": source, "samples": []} for name, source in sources.items()},
    }
    output = ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    for repeat in range(args.repeats):
        for name, source in sources.items():
            sample = run_case(name, source, repeat, args.timeout, ROOT / "_bench/warm-comparison")
            report["results"][name]["samples"].append(sample)
            if sample["status"] == "ok" and "trace" in sample:
                sample["api_ms"] = api_durations(json.loads((ROOT / sample["trace"]).read_text()))
                plugin = sample["events_ms"]["call plugin"]
                if len(sample["api_ms"]) != args.calls or len(plugin) != args.calls:
                    raise ValueError(f"{name}: missing calls, possibly memoized")
                if any(a < p for a, p in zip(sample["api_ms"], plugin)):
                    raise ValueError(f"{name}: API span does not contain plugin span")
            else:
                output.write_text(json.dumps(report, indent=2) + "\n")
                raise SystemExit(f"{name}: missing trace or failed compile: {sample['stderr']}")
            output.write_text(json.dumps(report, indent=2) + "\n")
            print(f"{repeat + 1}/{args.repeats} {name}: {sample['wall_ms']:.1f} ms wall", flush=True)
    report["plugins"] = {}
    for name, path in (
        ("fast-layout:0.1.0", ROOT / "fast-layout/plugin/fast_layout_engine.wasm"),
        ("diagraph-layout:0.0.1", args.diagraph_package / "graphviz_interface/diagraph.wasm"),
    ):
        data = path.read_bytes()
        report["plugins"][name] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    for name, result in report["results"].items():
        samples = result["samples"]
        result["warm_api_median_ms"] = statistics.median(v for s in samples for v in s["api_ms"][1:])
        result["warm_plugin_median_ms"] = statistics.median(
            v for s in samples for v in s["events_ms"]["call plugin"][1:]
        )
        print(f"{name}: API {result['warm_api_median_ms']:.3f} ms; plugin {result['warm_plugin_median_ms']:.3f} ms")
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    main()
