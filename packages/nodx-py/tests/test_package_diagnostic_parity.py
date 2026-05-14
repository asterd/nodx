"""Package-reader diagnostic parity (Rust ↔ JS ↔ Py).

The Rust reference implementation surfaces every package failure as
`PackageDiagnostic { code, severity, message }`. JavaScript mirrors this via
`NodxDiagnosticError`. This test pins the Python side to the same shape so a
single set of fixtures can be re-used across implementations.

If you add a new failure mode in `crates/nodx-package`, mirror the code/
severity here. See `spec/tests/security/README.md` for the broader contract.
"""

import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from nodx import open_stored_package  # noqa: E402
from nodx.package_diagnostics import NodxDiagnosticError, severity_for  # noqa: E402


def test_severity_table_matches_rust():
    # Mirrors `crates/nodx-package/src/lib.rs::severity_for`.
    assert severity_for("NODX-E010") == "error"
    assert severity_for("NODX-E021") == "error"
    assert severity_for("NODX-E019") == "fatal"
    assert severity_for("NODX-E012") == "fatal"  # default
    assert severity_for("NODX-E099") == "fatal"  # unknown -> default


def test_not_packaged_returns_diagnostic_not_value_error():
    try:
        open_stored_package(b"plain text not a zip")
    except NodxDiagnosticError as exc:
        assert exc.code == "NODX-E012"
        assert exc.severity == "fatal"
        # Structured form must be machine-readable.
        diag = exc.to_diagnostic()
        assert diag["code"] == "NODX-E012"
        assert diag["severity"] == "fatal"
    except ValueError:
        raise AssertionError(
            "package reader must raise NodxDiagnosticError, not ValueError; "
            "see crates/nodx-package/src/lib.rs::PackageDiagnostic"
        )


def test_unsafe_path_uses_e010():
    bytes_ = _build_zip(
        [
            ("mimetype", b"application/nodx+zip"),
            ("manifest.yaml", b"schema: nodx-package/1.0\nentry: ../escape.nodx\n"),
            ("../escape.nodx", b"# A\n"),
        ]
    )
    try:
        open_stored_package(bytes_)
    except NodxDiagnosticError as exc:
        assert exc.code == "NODX-E010"
        assert exc.severity == "error"
    else:
        raise AssertionError("expected NODX-E010 for unsafe path")


# Minimal in-process stored-ZIP builder for the fixtures above. Mirrors the
# helper used by `test_conformance.py` so tests stay readable.
def _build_zip(entries):
    chunks = []
    central = []
    offset = 0
    for name, data in entries:
        name_bytes = name.encode("utf-8")
        crc = zlib.crc32(data) & 0xFFFFFFFF
        local = struct.pack("<IHHHHHIIIHH",
                            0x04034B50, 20, 0, 0, 0, 0,
                            crc, len(data), len(data),
                            len(name_bytes), 0)
        chunks.append(local + name_bytes + data)
        central.append(struct.pack("<IHHHHHHIIIHHHHHII",
                                   0x02014B50, 20, 20, 0, 0, 0, 0,
                                   crc, len(data), len(data),
                                   len(name_bytes), 0, 0, 0, 0,
                                   0o100644 << 16, offset)
                       + name_bytes)
        offset += len(local) + len(name_bytes) + len(data)
    body = b"".join(chunks)
    cd = b"".join(central)
    eocd = struct.pack("<IHHHHIIH",
                       0x06054B50, 0, 0,
                       len(entries), len(entries),
                       len(cd), offset, 0)
    return body + cd + eocd
