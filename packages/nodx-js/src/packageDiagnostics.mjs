// Structured package-reader diagnostic.
//
// The Rust reference implementation surfaces every package-level failure as
// a `PackageDiagnostic { code, severity, message }`. This module mirrors that
// shape so JavaScript callers can branch on the code (NODX-E010, E012, E021)
// instead of pattern-matching English strings — which is what the parity
// audit flagged as the largest gap.
//
// Default code is NODX-E012 (resource limit / malformed envelope), matching
// the Rust `diag()` helper. Severity is derived from the code in the same way
// as `crates/nodx-package/src/lib.rs::severity_for`.

export class NodxDiagnosticError extends Error {
  constructor({ code, severity, message }) {
    super(`${code}: ${message}`);
    this.name = "NodxDiagnosticError";
    this.code = code;
    this.severity = severity;
    this.diagnosticMessage = message;
  }

  toDiagnostic() {
    return {
      code: this.code,
      severity: this.severity,
      message: this.diagnosticMessage,
      line: null,
      column: null,
      target: null,
    };
  }
}

export function severityFor(code) {
  switch (code) {
    case "NODX-E010":
    case "NODX-E021":
      return "error";
    case "NODX-E019":
      return "fatal";
    default:
      return "fatal";
  }
}

export function throwDiag(code, message) {
  throw new NodxDiagnosticError({ code, severity: severityFor(code), message });
}
