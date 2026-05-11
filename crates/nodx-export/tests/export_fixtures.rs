use nodx_core::parse_str;
use nodx_export::{export_docx, export_pdf_bridge, export_pptx, loss_report_json};

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
