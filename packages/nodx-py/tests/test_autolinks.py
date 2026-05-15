"""Autolink parity (RFC §12, PR3).

`<scheme:body>` and `<email>` produce an `Inline::Link` identical to the
explicit `[label](target)` form. URL safety stays single-sourced in
`nodx-url` at validate/render time — the parser is deliberately policy-free
so the conformance triplet (Rust/JS/Py) cannot disagree about *whether* a
link exists.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from nodx import canonical_json, parse  # noqa: E402


def test_https_autolink_produces_link():
    doc = parse("# T\n\nSee <https://example.com>.\n")
    rendered = canonical_json(doc)
    assert '"type":"link"' in rendered
    assert '"target":"https://example.com"' in rendered


def test_email_autolink_prefixes_mailto():
    doc = parse("# T\n\nMail <foo@bar.com>.\n")
    rendered = canonical_json(doc)
    assert '"target":"mailto:foo@bar.com"' in rendered


def test_mailto_autolink_passes_through_verbatim():
    doc = parse("# T\n\nMail <mailto:foo@bar.com>.\n")
    rendered = canonical_json(doc)
    assert '"target":"mailto:foo@bar.com"' in rendered
    # No double prefix when the body already has a scheme.
    assert "mailto:mailto:" not in rendered


def test_autolink_with_spaces_is_literal_text():
    doc = parse("# T\n\nNot <not a url> here.\n")
    rendered = canonical_json(doc)
    assert '"type":"link"' not in rendered


def test_javascript_autolink_is_still_a_link_node():
    # Parser is policy-free; validator/renderer rejects it.
    doc = parse("# T\n\nBad <javascript:alert(1)> stays a link node.\n")
    rendered = canonical_json(doc)
    assert '"target":"javascript:alert(1)"' in rendered


def test_consecutive_autolinks_parse_independently():
    doc = parse("# T\n\nHit <https://a.example> and <mailto:b@example.com>.\n")
    rendered = canonical_json(doc)
    assert '"target":"https://a.example"' in rendered
    assert '"target":"mailto:b@example.com"' in rendered


def test_autolink_full_url_path_query_fragment():
    doc = parse("# T\n\nURL <https://example.com/path?q=1#frag> here.\n")
    rendered = canonical_json(doc)
    assert '"target":"https://example.com/path?q=1#frag"' in rendered


def test_empty_brackets_are_literal_text():
    doc = parse("# T\n\nEmpty <> here.\n")
    rendered = canonical_json(doc)
    assert '"type":"link"' not in rendered
