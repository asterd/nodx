"""Text-side resource limits parity with `nodx_core::parse_str_with_limits`.

Before this contract was enforced, JS and Py parsers were best-effort on
hostile plain-text input — only package-related limits were checked. The
audit flagged the gap; the limits below are now active so the three
implementations agree on what counts as "too big to parse".
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from nodx import parse  # noqa: E402


def test_source_bytes_limit_emits_e012():
    doc = parse("A" * 100, limits={
        "sourceBytes": 10,
        "lineLength": 1024 * 1024,
        "frontMatterBytes": 64 * 1024,
    })
    assert any(d["code"] == "NODX-E012" and d["severity"] == "fatal" for d in doc["diagnostics"])
    assert doc["body"] == []


def test_line_length_limit_emits_e012():
    doc = parse("A" * 100, limits={
        "sourceBytes": 1024 * 1024,
        "lineLength": 10,
        "frontMatterBytes": 64 * 1024,
    })
    assert any(d["code"] == "NODX-E012" for d in doc["diagnostics"])


def test_front_matter_byte_limit_emits_e012():
    src = "---\n" + "k: v\n" * 50 + "---\n\nbody\n"
    doc = parse(src, limits={
        "sourceBytes": 1024 * 1024,
        "lineLength": 1024 * 1024,
        "frontMatterBytes": 10,
    })
    assert any(d["code"] == "NODX-E012" for d in doc["diagnostics"])
    assert doc["body"] == []


def test_within_limits_parses_normally():
    doc = parse("# Hello\n")
    assert doc["schema"] == "nodx/1.0"
    assert doc["body"]
    assert all(d["severity"] != "fatal" for d in doc["diagnostics"])
