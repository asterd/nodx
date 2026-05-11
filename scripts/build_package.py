#!/usr/bin/env python3
"""Build the bundled Packaged NODX example.

The generated file still uses `.nodx`; readers sniff ZIP magic bytes to detect
the package representation.
"""

from __future__ import annotations

import base64
import hashlib
import sys
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MIMETYPE = b"application/nodx+zip"

EDITORIAL_NODS = b"""heading[level="1"] {
  font-size: 30pt;
  font-weight: 700;
  margin-after: 12pt;
}

note[type="info"] {
  display: callout;
  border-start: 4pt solid #0f766e;
  background-color: #ecfdf5;
}

legal-clause {
  display: callout;
  border-start: 4pt solid #7c3aed;
  background-color: #f5f3ff;
}

approval-card[status="pending"] {
  display: callout;
  border-start: 4pt solid #b45309;
  background-color: #fff7ed;
}
"""

COMPONENTS_NODC = b"""{
  "schema": "nodx-components/0.1",
  "components": [
    {"name": "legal-clause", "version": "0.1.0", "fallback": "children"},
    {"name": "agent-review", "version": "0.1.0", "fallback": "children"},
    {"name": "approval-card", "version": "0.1.0", "fallback": "children"}
  ]
}
"""

PUBLIC_JWK = b"""{
  "kty": "EC",
  "crv": "P-256",
  "kid": "example-editor-key",
  "use": "sig",
  "alg": "ES256",
  "x": "BASE64URL_EXAMPLE_X",
  "y": "BASE64URL_EXAMPLE_Y"
}
"""

CHANGES_JSONL = (
    b'{"schema":"nodx/change/0.1","op":"set-attribute","target":"#approval",'
    b'"author":"agent:reviewer","time":"2026-05-10T10:00:00Z",'
    b'"reason":"Mark approval state","beforeHash":"sha256-demo-before",'
    b'"afterHash":"sha256-demo-after","requiresApproval":true}\n'
)

DOCUMENT_JWS = (
    b"eyJhbGciOiJFUzI1NiIsInR5cCI6Im5vZHgtc2lnbmF0dXJlK2p3cyIsImN0eSI6"
    b"ImFwcGxpY2F0aW9uL25vZHgtY2Fub25pY2FsK2pzb24ifQ.demo-payload.demo-signature\n"
)


def digest(data: bytes) -> str:
    raw = hashlib.sha256(data).digest()
    return "sha256-" + base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")


def media_type(path: str) -> str:
    if path.endswith(".nodx"):
        return "text/nodx"
    if path.endswith(".nods"):
        return "text/nodx-style"
    if path.endswith(".nodc"):
        return "application/nodx-components+json"
    if path.endswith(".svg"):
        return "image/svg+xml"
    if path.endswith(".jwk"):
        return "application/jwk+json"
    if path.endswith(".jsonl"):
        return "application/jsonl"
    if path.endswith(".jws"):
        return "application/jose"
    return "text/plain"


def collect_entries() -> list[tuple[str, bytes]]:
    return [
        ("content/document.nodx", (ROOT / "examples/extended-showcase.nodx").read_bytes()),
        ("styles/editorial.nods", EDITORIAL_NODS),
        ("components/editorial-components.nodc", COMPONENTS_NODC),
        ("assets/reference-pipeline.svg", (ROOT / "examples/assets/reference-pipeline.svg").read_bytes()),
        ("assets/demo-placeholder.txt", (ROOT / "examples/assets/demo-placeholder.txt").read_bytes()),
        ("keys/editor-public.jwk", PUBLIC_JWK),
        ("history/changes.jsonl", CHANGES_JSONL),
        ("signatures/document.jws", DOCUMENT_JWS),
    ]


def build(out_path: Path) -> None:
    entries = collect_entries()
    manifest_lines = [
        "schema: nodx-package/1.0",
        "entry: content/document.nodx",
        "signature: signatures/document.jws",
        "profiles:",
        "requires:",
        "  - core",
        "  - rich",
        "  - style",
        "optional:",
        "  - package",
        "entries:",
    ]
    for path, data in entries:
        manifest_lines.extend(
            [
                f"  - path: {path}",
                f"    size: {len(data)}",
                f"    sha256: {digest(data)}",
            ]
        )
    manifest = ("\n".join(manifest_lines) + "\n").encode()

    with zipfile.ZipFile(out_path, "w", compression=zipfile.ZIP_STORED) as zf:
        write_stored(zf, "mimetype", MIMETYPE)
        write_stored(zf, "manifest.yaml", manifest)
        for path, data in entries:
            write_stored(zf, path, data)


def write_stored(zf: zipfile.ZipFile, path: str, data: bytes) -> None:
    info = zipfile.ZipInfo(path)
    info.date_time = (2026, 5, 10, 0, 0, 0)
    info.compress_type = zipfile.ZIP_STORED
    info.external_attr = 0o100644 << 16
    zf.writestr(info, data)


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) == 2 else ROOT / "examples/extended-showcase-bundled.nodx"
    build(out)
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
