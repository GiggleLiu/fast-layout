#!/usr/bin/env python3
"""Compile every complete Typst example in the two READMEs."""

from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

with tempfile.TemporaryDirectory(prefix="fast-layout-readme-") as directory:
    for readme in (ROOT / "README.md", ROOT / "fast-layout/README.md"):
        examples = re.findall(r"```typst\n(.*?)```", readme.read_text(), re.DOTALL)
        if not examples:
            raise SystemExit(f"No Typst examples found in {readme}")
        for index, source in enumerate(examples, 1):
            doc = Path(directory) / "example.typ"
            doc.write_text(source)
            subprocess.run(
                ["typst", "compile", str(doc), str(doc.with_suffix(".pdf"))],
                check=True,
            )
            print(f"{readme.relative_to(ROOT)}: example {index} passed")
