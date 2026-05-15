"""CommonMark compatibility warnings (W030..W035).

Mirrors `crates/nodx-core/src/tests.rs` and
`packages/nodx-js/test/commonmark-warnings.test.mjs`. The Rust suite is
authoritative; byte-stable parity across the three parsers is enforced
separately by `scripts/run_conformance.sh`.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from nodx import parse  # noqa: E402


def codes(doc):
    return [d["code"] for d in doc["diagnostics"]]


def test_w030_setext_heading_emits_warning():
    doc = parse("Title\n=====\n")
    assert "NODX-W030" in codes(doc)
    warning = next(d for d in doc["diagnostics"] if d["code"] == "NODX-W030")
    assert warning["severity"] == "warning"
    assert warning["line"] == 2


def test_w031_indented_code_block_fires_once_per_run():
    doc = parse("    fn main() {}\n    println!();\n\n    again\n")
    hits = [d for d in doc["diagnostics"] if d["code"] == "NODX-W031"]
    assert len(hits) == 2
    assert hits[0]["line"] == 1
    assert hits[1]["line"] == 4


def test_w032_inline_image_emits_warning():
    doc = parse("See ![logo](logo.png) here.\n")
    assert "NODX-W032" in codes(doc)


def test_w033_link_reference_definition_emits_warning():
    doc = parse("[ref]: https://example.test\n")
    assert "NODX-W033" in codes(doc)


def test_w033_inline_link_reference_emits_warning():
    doc = parse("Use [label][ref] here.\n")
    assert "NODX-W033" in codes(doc)


def test_w034_footnote_definition_emits_warning():
    doc = parse("[^fn]: footnote text.\n")
    assert "NODX-W034" in codes(doc)


def test_w035_html_entity_emits_warning():
    doc = parse("Use &amp; and &#x76; and &#33; here.\n")
    hits = [d for d in doc["diagnostics"] if d["code"] == "NODX-W035"]
    assert len(hits) == 3
    assert all(d["severity"] == "warning" for d in hits)


def test_commonmark_warnings_are_non_fatal():
    doc = parse("Title\n=====\n\n    code\n\n[ref]: x\n")
    assert not any(d["severity"] == "fatal" for d in doc["diagnostics"])
    assert not any(d["severity"] == "error" for d in doc["diagnostics"])
