#!/usr/bin/env python3
"""Reproducible scalability benchmark for the NODX CLI.

The script uses only the Python standard library. It generates deterministic
synthetic books, runs selected `nodx` operations, prints a JSON report, and can
optionally fail on large regressions with `--assert-baseline`.
"""

from __future__ import annotations

import argparse
import json
import platform
import random
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PAGES = [2000, 5000, 10000]
DEFAULT_OPS = ["html"]


def sentence(rng: random.Random, n: int = 12) -> str:
    words = "structured document format deterministic canonical".split()
    return " ".join(rng.choice(words) for _ in range(n)).capitalize() + "."


def paragraph(rng: random.Random, n: int = 4) -> str:
    return " ".join(sentence(rng) for _ in range(n))


def generate_book(path: Path, pages: int) -> None:
    rng = random.Random(42)
    with path.open("w", encoding="utf-8") as handle:
        handle.write("---\ntitle: Synthetic\n---\n\n")
        for page in range(1, pages + 1):
            handle.write(f"## Chapter {page} #ch{page}\n\n{paragraph(rng)}\n\n")
            handle.write(f"### Section {page}.1 #ch{page}-s1\n\n{paragraph(rng)}\n\n")
            handle.write("- " + sentence(rng) + "\n- " + sentence(rng) + "\n\n")
            handle.write("| A | B |\n|---|---|\n| 1 | 2 |\n\n")


def run_command(command: list[str]) -> tuple[float, int | None]:
    if platform.system() != "Darwin":
        return run_command_with_time(command)
    started = time.perf_counter()
    result = subprocess.run(
        command,
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    elapsed = time.perf_counter() - started
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    return elapsed, None


def run_command_with_time(command: list[str]) -> tuple[float, int | None]:
    timed = ["/usr/bin/time", "-v", *command]
    started = time.perf_counter()
    result = subprocess.run(
        timed,
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    elapsed = time.perf_counter() - started
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    match = re.search(r"Maximum resident set size.*:\s+(\d+)", result.stderr)
    rss_mib = round(int(match.group(1)) / 1024) if match else None
    return elapsed, rss_mib


def assert_baseline(entries: list[dict[str, object]]) -> None:
    for entry in entries:
        pages = int(entry["pages"])
        op = str(entry["operation"])
        seconds = float(entry["wallSeconds"])
        rss_mib = entry.get("peakRssMiB")
        if op == "html":
            max_seconds = max(2.0, pages * 0.00025)
            if seconds > max_seconds:
                raise SystemExit(
                    f"benchmark regression: html {pages} pages took {seconds:.3f}s > {max_seconds:.3f}s"
                )
            if isinstance(rss_mib, int):
                max_rss = max(160, pages * 0.06)
                if rss_mib > max_rss:
                    raise SystemExit(
                        f"benchmark regression: html {pages} pages used {rss_mib} MiB > {max_rss:.0f} MiB"
                    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/release/nodx")
    parser.add_argument("--pages", nargs="+", type=int, default=DEFAULT_PAGES)
    parser.add_argument("--ops", nargs="+", default=DEFAULT_OPS)
    parser.add_argument("--output", default="target/scalability-report.json")
    parser.add_argument("--assert-baseline", action="store_true")
    parser.add_argument("--no-build", action="store_true")
    args = parser.parse_args()

    binary = ROOT / args.binary
    if not args.no_build:
        subprocess.run(["cargo", "build", "--release", "-p", "nodx"], cwd=ROOT, check=True)
    if not binary.exists():
        raise SystemExit(f"missing binary: {binary}")

    entries: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="nodx-bench-") as tmp:
        tmpdir = Path(tmp)
        for pages in args.pages:
            source = tmpdir / f"book-{pages}.nodx"
            generate_book(source, pages)
            source_bytes = source.stat().st_size
            for op in args.ops:
                elapsed, rss_mib = run_command([str(binary), op, str(source)])
                entry: dict[str, object] = {
                    "operation": op,
                    "pages": pages,
                    "sourceBytes": source_bytes,
                    "wallSeconds": round(elapsed, 4),
                    "peakRssMiB": rss_mib,
                }
                entries.append(entry)

    report = {
        "schema": "nodx/scalability-benchmark/1.0",
        "host": platform.platform(),
        "entries": entries,
    }
    output = ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    if args.assert_baseline:
        assert_baseline(entries)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
