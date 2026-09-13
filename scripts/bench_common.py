"""Shared graph and machine metadata for the layout benchmarks."""

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


