# NODX Export

`nodx-export` implements the NODX Presentation 1.2 exporter baseline.

The PDF path intentionally emits a safe paged HTML host bridge instead of a
native PDF renderer. DOCX and PPTX outputs are minimal stored-ZIP Office Open
XML packages. All lossy exports return a deterministic machine-readable loss
report that callers should persist beside the exported artifact.

External validation is best-effort and remains outside this crate so exporter
tests can run without network access or system office tools.
