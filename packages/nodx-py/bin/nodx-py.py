#!/usr/bin/env python3
"""Conformance CLI for the Python reference parser.

Mirrors `packages/nodx-js/bin/nodx-js.mjs` so `scripts/run_conformance.sh`
can drive Rust ↔ JS ↔ Py byte-for-byte parity through the same fixture loop.

Usage:
    nodx-py.py <ast|ncp|semantic|diagnostics> <file.nodx>
"""
import sys
from pathlib import Path

# Allow running from a checkout without installing the package.
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "src"))

from nodx import (  # noqa: E402
    canonical_json,
    diagnostics_json,
    exit_code_for,
    ncp_json,
    parse,
    render_semantic_text,
    validate,
)


def main(argv):
    if len(argv) != 3 or argv[1] not in ("ast", "ncp", "semantic", "diagnostics"):
        sys.stderr.write(
            "usage: nodx-py.py <ast|ncp|semantic|diagnostics> <file.nodx>\n"
        )
        return 2
    command, path = argv[1], argv[2]
    source = Path(path).read_text(encoding="utf-8")
    doc = parse(source)
    if command == "ast":
        sys.stdout.write(canonical_json(doc) + "\n")
        return 0
    if command == "ncp":
        sys.stdout.write(ncp_json(doc) + "\n")
        return 0
    if command == "semantic":
        sys.stdout.write(render_semantic_text(doc))
        return 0
    diagnostics = validate(doc)
    sys.stdout.write(diagnostics_json(diagnostics) + "\n")
    return exit_code_for(diagnostics)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
