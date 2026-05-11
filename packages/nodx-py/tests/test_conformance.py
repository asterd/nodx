import json
import subprocess
from pathlib import Path

from nodx import (
    audit_stylesheet,
    canonical_json,
    diagnostics_json,
    ncp_json,
    open_stored_package,
    package_entry_text,
    parse,
    render_html,
    render_semantic_text,
    theme_stylesheet,
    validate,
)


ROOT = Path(__file__).resolve().parents[3]


def positive_fixtures():
    dirs = [
        "spec/tests/conformance",
        "spec/tests/ncp",
        "spec/tests/navigation",
        "spec/tests/rendering",
    ]
    out = []
    for dir_ in dirs:
        out.extend(sorted((ROOT / dir_).glob("*.nodx")))
    out.extend(
        [
            ROOT / "spec/conformance/v1.0/fixtures/minimal.nodx",
            ROOT / "spec/conformance/v1.0/fixtures/rich-web.nodx",
        ]
    )
    return out


def test_ast_expected_fixtures():
    for fixture in (ROOT / "spec/conformance/v1.0/fixtures").glob("*.nodx"):
        expected = ROOT / "spec/conformance/v1.0/expected" / fixture.name.replace(".nodx", ".ast.json")
        if not expected.exists():
            continue
        assert canonical_json(parse(fixture.read_text())) == expected.read_text().strip()


def test_ncp_expected_fixtures():
    for fixture in (ROOT / "spec/conformance/v1.0/fixtures").glob("*.nodx"):
        expected = ROOT / "spec/conformance/v1.0/expected" / fixture.name.replace(".nodx", ".ncp.json")
        if not expected.exists():
            continue
        assert json.loads(ncp_json(parse(fixture.read_text()))) == json.loads(expected.read_text())


def test_python_matches_rust_ast_and_ncp_when_built():
    rust = ROOT / "target/debug/nodx"
    if not rust.exists():
        return
    for fixture in positive_fixtures():
        rel = fixture.relative_to(ROOT).as_posix()
        doc = parse(fixture.read_text())
        rust_ast = subprocess.check_output([rust, "ast", rel], cwd=ROOT, text=True)
        assert canonical_json(doc) + "\n" == rust_ast
        rust_ncp = subprocess.check_output([rust, "ncp", rel], cwd=ROOT, text=True)
        assert ncp_json(doc) + "\n" == rust_ncp


def test_negative_diagnostics_golden_subset():
    for expected in sorted((ROOT / "spec/tests/golden").glob("*.diagnostics.json")):
        fixture = ROOT / "spec/tests/negative" / expected.name.replace(".diagnostics.json", ".nodx")
        if not fixture.exists():
            continue
        got = diagnostics_json(validate(parse(fixture.read_text())))
        want = expected.read_text().rstrip()
        if "NODX-E027" in want:
            continue
        assert got == want


def test_package_reader_matches_js_surface():
    bytes_ = (ROOT / "examples/extended-showcase-bundled.nodx").read_bytes()
    pkg = open_stored_package(bytes_)
    assert pkg.entry_path == "content/document.nodx"
    assert "assets/reference-pipeline.svg" in pkg.files
    assert "Extended Showcase" in package_entry_text(bytes_)
    assert parse(package_entry_text(bytes_))["schema"] == "nodx/1.0"


def test_renderer_theme_and_semantic_text():
    doc = parse((ROOT / "examples/showcase-web.nodx").read_text())
    html = render_html(doc)
    assert html.startswith("<!doctype html>")
    assert "<body>" in html
    assert "nodx-blocked-link" not in html
    assert "--nodx-color-text" in theme_stylesheet("web")
    assert render_semantic_text(doc).endswith("\n")


def test_nods_hostile_corpus():
    checked = 0
    for fixture in sorted((ROOT / "spec/tests/security/nods-hostile").glob("*.nodx")):
        doc = parse(fixture.read_text())
        diagnostics = validate(doc)
        html = render_html(doc).lower()
        if fixture.name == "safe-subset.nodx":
            assert not any(d["code"] == "NODX-E027" and d["severity"] == "error" for d in diagnostics)
        else:
            assert any(d["code"] == "NODX-E027" and d["severity"] == "error" for d in diagnostics), fixture
        assert "<script" not in html
        assert "<iframe" not in html
        assert "javascript:" not in html
        checked += 1
    assert checked >= 28


def test_nods_audit_and_sanitize():
    assert audit_stylesheet("h1{color:red}") == []
    assert audit_stylesheet("a:hover{color:red}")
