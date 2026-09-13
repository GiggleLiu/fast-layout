"""Shared graph, metadata, and timing helpers for layout benchmarks."""

import json
import os
import time
import platform
import random
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


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


EVENTS = ("compile once", "load plugin", "create plugin instance", "call plugin")


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


def run_case(name, source, repeat, timeout, output_dir):
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
