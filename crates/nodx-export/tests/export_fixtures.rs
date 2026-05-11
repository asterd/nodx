use nodx_core::{ResourceLimits, parse_str};
use nodx_export::{
    ExportFormat, export_docx, export_docx_with_limits, export_document_with_limits,
    export_pdf_bridge, export_pptx, loss_report_json,
};

#[test]
fn presentation_fixture_exports_to_pptx() {
    let doc = parse_str(include_str!(
        "../../../spec/tests/presentation/presentation-basic.nodx"
    ));
    let exported = export_pptx(&doc);
    assert!(exported.bytes.starts_with(b"PK\x03\x04"));
    assert_eq!(
        loss_report_json(&exported.loss_report),
        include_str!("../../../spec/tests/presentation/presentation-basic.pptx.loss.json")
            .trim_end()
    );
}

#[test]
fn lossy_fixture_reports_docx_losses() {
    let doc = parse_str(include_str!("../../../spec/tests/export/lossy-export.nodx"));
    let exported = export_docx(&doc);
    assert!(exported.bytes.starts_with(b"PK\x03\x04"));
    assert_eq!(
        loss_report_json(&exported.loss_report),
        include_str!("../../../spec/tests/export/lossy-export.docx.loss.json").trim_end()
    );
}

#[test]
fn export_resource_limits_are_enforced() {
    let body = "# Title\n\nLong body. ".repeat(1);
    let doc = parse_str(&body);
    let limits = ResourceLimits {
        export_bytes: 1,
        ..ResourceLimits::default()
    };
    let exported = export_document_with_limits(&doc, ExportFormat::Docx, limits);
    assert!(exported.bytes.is_empty(), "limits should clamp the export");
}

#[test]
fn export_strips_xml_invalid_control_characters() {
    // \u{0007} (BEL) is not allowed in XML 1.0 — exporter must elide it.
    let doc = parse_str(":::slide {title=\"a\\u0007b\"}\n# Slide\nBody\n:::\n");
    let exported = export_docx_with_limits(&doc, ResourceLimits::default());
    let xml = String::from_utf8_lossy(&exported.bytes);
    assert!(!xml.contains('\u{0007}'));
}

#[test]
fn pdf_fixture_uses_paged_html_bridge() {
    let doc = parse_str(include_str!("../../../spec/tests/export/lossy-export.nodx"));
    let exported = export_pdf_bridge(&doc);
    let html = String::from_utf8(exported.bytes).expect("html bytes");
    assert!(html.contains("nodx-pdf-bridge"));
    assert!(html.contains("@page"));
    assert_eq!(
        loss_report_json(&exported.loss_report),
        include_str!("../../../spec/tests/export/lossy-export.pdf.loss.json").trim_end()
    );
}
