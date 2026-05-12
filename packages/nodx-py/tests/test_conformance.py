import json
import struct
import subprocess
import zlib
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


def test_package_reader_rejects_digest_mismatch():
    data = b"# A\n"
    bytes_ = build_zip(
        [
            ("mimetype", b"application/nodx+zip"),
            ("manifest.yaml", package_manifest("doc.nodx", data, "sha256-bad").encode()),
            ("doc.nodx", data),
        ]
    )
    try:
        open_stored_package(bytes_)
    except ValueError as exc:
        assert "digest mismatch" in str(exc)
    else:
        raise AssertionError("expected digest mismatch")


def test_package_reader_rejects_percent_encoded_traversal():
    bytes_ = build_zip(
        [
            ("mimetype", b"application/nodx+zip"),
            ("manifest.yaml", b"schema: nodx-package/1.0\nentry: a/%2e%2e/doc.nodx\n"),
            ("a/%2e%2e/doc.nodx", b"# A\n"),
        ]
    )
    try:
        open_stored_package(bytes_)
    except ValueError as exc:
        assert "unsafe package path" in str(exc)
    else:
        raise AssertionError("expected unsafe path")


def test_package_reader_rejects_duplicate_entries():
    data = b"# A\n"
    bytes_ = build_zip(
        [
            ("mimetype", b"application/nodx+zip"),
            ("manifest.yaml", package_manifest("doc.nodx", data).encode()),
            ("doc.nodx", data),
            ("doc.nodx", b"# B\n"),
        ]
    )
    try:
        open_stored_package(bytes_)
    except ValueError as exc:
        assert "duplicate package entry path" in str(exc)
    else:
        raise AssertionError("expected duplicate entry")


def test_renderer_theme_and_semantic_text():
    doc = parse((ROOT / "examples/showcase-web.nodx").read_text())
    html = render_html(doc)
    assert html.startswith("<!doctype html>")
    assert "<body>" in html
    assert '<a class="nodx-blocked-link"' not in html
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


def package_manifest(path, data, digest=None):
    from nodx.bytes import sha256_base64_url

    digest = digest or sha256_base64_url(data)
    return f"""schema: nodx-package/1.0
entry: {path}
entries:
  - path: {path}
    size: {len(data)}
    sha256: {digest}
"""


def build_zip(entries):
    out = bytearray()
    central = []
    for name, data in entries:
        name_bytes = name.encode()
        crc = zlib.crc32(data) & 0xFFFFFFFF
        local_offset = len(out)
        out.extend(struct.pack("<IHHHHHIIIHH", 0x04034B50, 20, 0, 0, 0, 0, crc, len(data), len(data), len(name_bytes), 0))
        out.extend(name_bytes)
        out.extend(data)
        central.append((name_bytes, data, crc, local_offset))
    cd_offset = len(out)
    for name_bytes, data, crc, local_offset in central:
        out.extend(
            struct.pack(
                "<IHHHHHHIIIHHHHHII",
                0x02014B50,
                20,
                20,
                0,
                0,
                0,
                0,
                crc,
                len(data),
                len(data),
                len(name_bytes),
                0,
                0,
                0,
                0,
                0o100644 << 16,
                local_offset,
            )
        )
        out.extend(name_bytes)
    cd_size = len(out) - cd_offset
    out.extend(struct.pack("<IHHHHIIH", 0x06054B50, 0, 0, len(central), len(central), cd_size, cd_offset, 0))
    return bytes(out)
