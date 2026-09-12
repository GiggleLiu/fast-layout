#!/usr/bin/env python3
"""Measure initial and incremental Typst watch compilations."""

import argparse
import hashlib
import json
import os
import pty
import re
import selectors
import subprocess
import tempfile
import time
from pathlib import Path

from bench_typst import ROOT, graph, machine_info, typst_array

OUT = ROOT / "_bench"


def source(nodes, algorithm, seed, note):
    if algorithm == "shell":
        edges = []
    elif algorithm == "buchheim":
        edges = [((i - 1) // 2, i) for i in range(1, nodes)]
    else:
        edges = graph(nodes)
    return (
        '#import "@preview/fast-layout:0.1.0": layout\n'
        f'#let result = layout({nodes}, {typst_array(edges)}, algorithm: "{algorithm}", seed: {seed})\n'
        f'#let note = "{note}"\n'
        "#assert.eq(result.positions.len(), " + str(nodes) + ")\n"
        "#note\n"
    )


def trace_compile_ms(events):
    starts = []
    durations = []
    for event in events:
        if event.get("name") == "compile once":
            if event.get("ph") == "B":
                starts.append(event["ts"])
            elif event.get("ph") == "E" and starts:
                durations.append((event["ts"] - starts.pop()) / 1000.0)
    if not durations:
        raise ValueError("timing trace has no completed compile once event")
    return durations[-1]


def duration_ms(value, unit):
    return float(value) * {"µs": 0.001, "us": 0.001, "ms": 1.0, "s": 1000.0}[unit]


def wait_compile(output_fd, directory, timeout, started):
    deadline = started + timeout
    selector = selectors.DefaultSelector()
    selector.register(output_fd, selectors.EVENT_READ)
    output = ""
    while time.perf_counter() < deadline:
        for _, _ in selector.select(timeout=min(0.05, deadline - time.perf_counter())):
            try:
                output += os.read(output_fd, 4096).decode(errors="replace")
            except OSError as error:
                raise RuntimeError(f"Typst watch closed its terminal: {output}") from error
            match = re.search(r"compiled successfully in ([0-9.]+)\s*(µs|us|ms|s)", output)
            if match:
                result = {
                    "wall_ms": (time.perf_counter() - started) * 1000.0,
                    "compile_ms": duration_ms(*match.groups()),
                }
                try:
                    trace = max(directory.glob("timings-*.json"), key=lambda path: path.stat().st_mtime_ns)
                    events = json.loads(trace.read_text())
                    result["trace_compile_ms"] = trace_compile_ms(events)
                    result["trace_events"] = len(events)
                    result["trace"] = trace.name
                except (FileNotFoundError, json.JSONDecodeError, ValueError):
                    pass
                selector.close()
                return result
            if "compiled with errors" in output:
                selector.close()
                raise RuntimeError(output)
    selector.close()
    raise TimeoutError(f"Typst watch did not finish within {timeout}s")


def one_run(nodes, algorithm, timeout, directory):
    doc = directory / "case.typ"
    pdf = directory / "case.pdf"
    trace = directory / "timings-{n}.json"
    doc.write_text(source(nodes, algorithm, 1, "first"))
    env = os.environ.copy()
    env["TYPST_PACKAGE_PATH"] = str(ROOT / "_pkgroot")
    master, slave = pty.openpty()
    started = time.perf_counter()
    process = subprocess.Popen(
        ["typst", "watch", "--root", str(ROOT), "--timings", str(trace), str(doc), str(pdf)],
        cwd=ROOT,
        env=env,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        text=True,
    )
    os.close(slave)
    try:
        first = wait_compile(master, directory, timeout, started)
        started = time.perf_counter()
        doc.write_text(source(nodes, algorithm, 1, "text changed"))
        text_only = wait_compile(master, directory, timeout, started)
        started = time.perf_counter()
        doc.write_text(source(nodes, algorithm, 2, "text changed"))
        seed_change = wait_compile(master, directory, timeout, started)
        return {"first_load": first, "text_only": text_only, "seed_change": seed_change}
    finally:
        process.terminate()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        os.close(master)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--algorithm", default="spring", choices=("stress", "spring", "spectral", "shell", "buchheim"))
    parser.add_argument("--nodes", type=int, default=500)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--output", default="docs/benchmarks/typst-incremental.json")
    args = parser.parse_args()
    if args.nodes < 1 or args.repeats < 1 or args.timeout <= 0:
        parser.error("nodes, repeats, and timeout must be positive")

    OUT.mkdir(exist_ok=True)
    runs = []
    for _ in range(args.repeats):
        with tempfile.TemporaryDirectory(prefix="incremental-", dir=OUT) as temp:
            runs.append(one_run(args.nodes, args.algorithm, args.timeout, Path(temp)))
    report = {
        "machine": machine_info(),
        "wasm_sha256": hashlib.sha256(
            (ROOT / "fast-layout/plugin/fast_layout_engine.wasm").read_bytes()
        ).hexdigest(),
        "settings": vars(args),
        "timing_note": "wall_ms includes file watching and debounce; compile_ms comes from the watch completion event; trace_compile_ms and trace_events are included when Typst rewrites --timings",
        "runs": runs,
    }
    output = ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    main()
