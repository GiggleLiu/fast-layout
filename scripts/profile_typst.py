#!/usr/bin/env python3
"""Profile 100-node fast-layout calls in fresh Typst processes."""

import argparse
import hashlib
import json
import os
import statistics
import subprocess
import time
from pathlib import Path

from bench_common import ROOT, graph, machine_info, typst_array

OUT = ROOT / "_bench/profile-100"
WASM = ROOT / "fast-layout/plugin/fast_layout_engine.wasm"
EVENTS = ("compile once", "load plugin", "create plugin instance", "call plugin")


def baseline_source(import_line="", extra=""):
    return (
        import_line
        + "#let result = range(100).map(i => (float(i), 0.0))\n"
        + "#assert.eq(result.len(), 100)\n"
        + extra
    )


def layout_source(
    algorithm, iterations=None, tolerance=None, calls=1, stress_method=None
):
    edges = () if algorithm == "shell" else graph(100)
    options = f', algorithm: "{algorithm}"'
    if iterations is not None:
        options += f", iterations: {iterations}"
    if tolerance is not None:
        options += f", tolerance: {tolerance}"
    if stress_method is not None:
        options += f', stress-method: "{stress_method}"'
    invocations = ",\n  ".join(
        f"layout(100, edges{options}, seed: {seed})" for seed in range(1, calls + 1)
    )
    return (
        '#import "@preview/fast-layout:0.1.0": layout\n'
        f"#let edges = {typst_array(edges)}\n"
        f"#let results = (\n  {invocations},\n)\n"
        f"#assert.eq(results.len(), {calls})\n"
        "#assert(results.all(result => result.positions.len() == 100))\n"
    )


def cases(repeated_calls):
    result = {
        "no-plugin": baseline_source(),
        "import-only": baseline_source('#import "@preview/fast-layout:0.1.0": layout\n'),
        "engine-version": baseline_source(
            '#import "@preview/fast-layout:0.1.0": engine-version\n',
            '#assert.eq(engine-version(), "fast-layout-engine 0.1.0")\n',
        ),
    }
    result["stress-default-one"] = layout_source("stress")
    result[f"stress-default-repeat-{repeated_calls}"] = layout_source(
        "stress", calls=repeated_calls
    )
    for algorithm in ("shell", "spectral"):
        result[f"{algorithm}-one"] = layout_source(algorithm)
        result[f"{algorithm}-repeat-{repeated_calls}"] = layout_source(
            algorithm, calls=repeated_calls
        )
    for algorithm in ("stress", "spring"):
        for iterations in (1, 5, 20, 100):
            for tolerance_name, tolerance in (("default", None), ("zero", 0.0)):
                stem = f"{algorithm}-iter-{iterations}-tol-{tolerance_name}"
                result[f"{stem}-one"] = layout_source(
                    algorithm, iterations, tolerance,
                    stress_method="majorization" if algorithm == "stress" else None,
                )
                result[f"{stem}-repeat-{repeated_calls}"] = layout_source(
                    algorithm, iterations, tolerance, repeated_calls,
                    stress_method="majorization" if algorithm == "stress" else None,
                )
    for iterations in (5, 15):
        stem = f"stress-sgd-iter-{iterations}"
        result[f"{stem}-one"] = layout_source(
            "stress", iterations, stress_method="sgd"
        )
        result[f"{stem}-repeat-{repeated_calls}"] = layout_source(
            "stress", iterations, calls=repeated_calls, stress_method="sgd"
        )
    return result


def trace_durations(events):
    stacks = {}
    durations = {name: [] for name in EVENTS}
    for event in events:
        name = event.get("name")
        if name not in durations:
            continue
        key = (event.get("pid"), event.get("tid"), name)
        if event.get("ph") == "B":
            stacks.setdefault(key, []).append(event["ts"])
        elif event.get("ph") == "E" and stacks.get(key):
            durations[name].append((event["ts"] - stacks[key].pop()) / 1000.0)
    return durations


def run_case(name, source, repeat, timeout, output_dir=OUT):
    directory = output_dir / name / f"run-{repeat + 1}"
    directory.mkdir(parents=True, exist_ok=True)
    doc = directory / "case.typ"
    doc.write_text(source)
    trace_pattern = directory / "timings-{n}.json"
    env = os.environ.copy()
    env["TYPST_PACKAGE_PATH"] = str(ROOT / "_pkgroot")
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            [
                "typst",
                "compile",
                "--root",
                str(ROOT),
                "--timings",
                str(trace_pattern),
                str(doc),
                str(directory / "out.pdf"),
            ],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        return {
            "status": "timeout",
            "wall_ms": (time.perf_counter() - started) * 1000,
            "stderr": (error.stderr or b"").decode(errors="replace")[-4000:],
        }
    sample = {
        "status": "ok" if proc.returncode == 0 else "error",
        "returncode": proc.returncode,
        "wall_ms": (time.perf_counter() - started) * 1000,
        "stderr": proc.stderr[-4000:],
    }
    traces = list(directory.glob("timings-*.json"))
    if proc.returncode == 0 and len(traces) == 1:
        events = json.loads(traces[0].read_text())
        durations = trace_durations(events)
        sample["trace"] = str(traces[0].relative_to(ROOT))
        sample["trace_event_count"] = len(events)
        sample["events_ms"] = durations
        sample["event_totals_ms"] = {
            event: sum(values) for event, values in durations.items()
        }
    return sample


def medians(samples):
    result = {"wall_ms": statistics.median(s["wall_ms"] for s in samples)}
    for event in EVENTS:
        values = [s["event_totals_ms"][event] for s in samples]
        result[f"{event.replace(' ', '_')}_total_ms"] = statistics.median(values)
    call_counts = [len(s["events_ms"]["call plugin"]) for s in samples]
    result["call_plugin_count"] = statistics.median(call_counts)
    per_call = [
        value
        for sample in samples
        for value in sample["events_ms"]["call plugin"]
    ]
    result["call_plugin_individual_median_ms"] = (
        statistics.median(per_call) if per_call else None
    )
    warmed_calls = [
        value
        for sample in samples
        for value in sample["events_ms"]["call plugin"][1:]
    ]
    result["call_plugin_warmed_median_ms"] = (
        statistics.median(warmed_calls) if warmed_calls else None
    )
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--repeated-calls", type=int, default=5)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument(
        "--output", default="_bench/profile-100.json"
    )
    args = parser.parse_args()
    if args.repeats < 1 or args.repeated_calls < 2 or args.timeout <= 0:
        parser.error("repeats must be positive, repeated-calls >= 2, timeout positive")

    OUT.mkdir(parents=True, exist_ok=True)
    all_cases = cases(args.repeated_calls)
    report = {
        "machine": machine_info(),
        "git_revision": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True
        ).stdout.strip(),
        "typst_source_sha256": hashlib.sha256(
            (ROOT / "fast-layout/src/engine.typ").read_bytes()
        ).hexdigest(),
        "wasm_sha256": hashlib.sha256(WASM.read_bytes()).hexdigest(),
        "wasm_bytes": WASM.stat().st_size,
        "settings": vars(args),
        "method": {
            "process": "fresh Typst process per sample; cases run round-robin",
            "output": "all documents render the same blank one-page PDF and assert 100 positions",
            "repeated_calls": "distinct seeds prevent Typst memoization",
            "timing": "raw wall and trace event durations; no baseline subtraction",
            "trace_limitation": "Typst 0.15.1 names plugin exports only as 'call plugin'; export identity is inferred from each controlled document",
        },
        "results": {name: {"samples": []} for name in all_cases},
    }
    output = ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    for repeat in range(args.repeats):
        for name, source in all_cases.items():
            sample = run_case(name, source, repeat, args.timeout)
            report["results"][name]["samples"].append(sample)
            output.write_text(json.dumps(report, indent=2) + "\n")
            print(f"{repeat + 1}/{args.repeats} {name}: {sample['status']} {sample['wall_ms']:.1f} ms")
            if sample["status"] != "ok":
                raise SystemExit(f"failed case {name}: {sample['stderr']}")
    for result in report["results"].values():
        result["median"] = medians(result["samples"])
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    main()
