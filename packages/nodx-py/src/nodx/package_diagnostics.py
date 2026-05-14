"""Structured package-reader diagnostic.

Mirrors `crates/nodx-package/src/lib.rs::PackageDiagnostic` so Python callers
can branch on `NODX-Exxx` codes instead of regexing English strings. The
default code is `NODX-E012` (resource limit / malformed envelope), matching
the Rust `diag()` helper. Severity follows `severity_for`.
"""


class NodxDiagnosticError(Exception):
    def __init__(self, code, message, severity=None):
        if severity is None:
            severity = severity_for(code)
        super().__init__(f"{code}: {message}")
        self.code = code
        self.severity = severity
        self.diagnostic_message = message

    def to_diagnostic(self):
        return {
            "code": self.code,
            "severity": self.severity,
            "message": self.diagnostic_message,
            "line": None,
            "column": None,
            "target": None,
        }


def severity_for(code):
    if code in ("NODX-E010", "NODX-E021"):
        return "error"
    if code == "NODX-E019":
        return "fatal"
    return "fatal"


def raise_diag(code, message):
    raise NodxDiagnosticError(code, message)
