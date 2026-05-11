use std::{env, fs, path::PathBuf, process};

use nodx_core::{canonical_json, is_packaged_nodx, ncp_json, parse_bytes, render_tui};
use nodx_export::{ExportFormat, export_document, loss_report_json};
use nodx_render_html::render_html;
use nodx_validate::{Validator, diagnostics_json, exit_code_for};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: nodx <ast|html|tui|ncp|diagnostics|validate|inspect> <file.nodx>\n       nodx export <pdf|docx|pptx> <file.nodx> -o <out>"
        );
        process::exit(1);
    }
    let command = args[1].as_str();
    if command == "export" {
        process::exit(export_command(&args[2..]));
    }
    let file = args[2].as_str();
    let options = parse_options(&args[3..]).unwrap_or_else(|err| {
        eprintln!("{err}");
        process::exit(1);
    });
    let bytes = fs::read(file).unwrap_or_else(|err| {
        eprintln!("read error: {err}");
        process::exit(1);
    });
    let exit = match command {
        "inspect" => {
            reject_options(&options);
            if is_packaged_nodx(&bytes) {
                println!("format: packaged-nodx");
                println!("container: zip");
            } else {
                println!("format: text-nodx");
                println!("container: utf-8");
            }
            0
        }
        "ast" => {
            reject_profile(&options);
            let doc = parse_or_exit(&bytes);
            println!("{}", canonical_json(&doc));
            diagnostic_exit_code(&doc)
        }
        "html" => {
            reject_options(&options);
            let doc = parse_or_exit(&bytes);
            println!("{}", render_html(&doc));
            diagnostic_exit_code(&doc)
        }
        "tui" => {
            reject_options(&options);
            let doc = parse_or_exit(&bytes);
            print!("{}", render_tui(&doc));
            diagnostic_exit_code(&doc)
        }
        "ncp" => {
            reject_options(&options);
            let doc = parse_or_exit(&bytes);
            println!("{}", ncp_json(&doc));
            diagnostic_exit_code(&doc)
        }
        "diagnostics" => {
            reject_profile(&options);
            let doc = parse_or_exit(&bytes);
            let diagnostics = Validator::default().validate(&doc);
            if options.format == "json" {
                println!("{}", diagnostics_json(&diagnostics));
            } else {
                print_diagnostics(&diagnostics);
            }
            exit_code_for(&diagnostics)
        }
        "validate" => {
            let doc = parse_or_exit(&bytes);
            let diagnostics =
                Validator::default().validate_with_profile(&doc, options.profile.as_deref());
            if options.format == "json" {
                println!("{}", diagnostics_json(&diagnostics));
            } else {
                print_diagnostics(&diagnostics);
            }
            exit_code_for(&diagnostics)
        }
        other => {
            eprintln!("unknown command: {other}");
            1
        }
    };
    if exit != 0 {
        process::exit(exit);
    }
}

fn export_command(args: &[String]) -> i32 {
    if args.len() != 4 || args[2] != "-o" {
        eprintln!("usage: nodx export <pdf|docx|pptx> <file.nodx> -o <out>");
        return 1;
    }
    let format = match args[0].as_str() {
        "pdf" => ExportFormat::Pdf,
        "docx" => ExportFormat::Docx,
        "pptx" => ExportFormat::Pptx,
        other => {
            eprintln!("unknown export format: {other}");
            return 1;
        }
    };
    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("read error: {err}");
            return 1;
        }
    };
    let doc = parse_or_exit(&bytes);
    let exported = export_document(&doc, format);
    if let Err(err) = fs::write(&args[3], &exported.bytes) {
        eprintln!("write error: {err}");
        return 1;
    }
    let report_path = loss_report_path(&args[3]);
    let mut report_json = loss_report_json(&exported.loss_report);
    report_json.push('\n');
    if let Err(err) = fs::write(&report_path, report_json) {
        eprintln!("loss report write error: {err}");
        return 1;
    }
    diagnostic_exit_code(&doc)
}

fn loss_report_path(output: &str) -> PathBuf {
    let mut path = PathBuf::from(output);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("export");
    path.set_file_name(format!("{file_name}.loss.json"));
    path
}

#[derive(Clone, Debug)]
struct Options {
    format: String,
    profile: Option<String>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        format: "text".to_string(),
        profile: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--format requires a value".to_string())?;
                if value != "text" && value != "json" {
                    return Err("--format must be text or json".to_string());
                }
                options.format = value.clone();
                i += 2;
            }
            "--profile" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                options.profile = Some(value.clone());
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(options)
}

fn reject_options(options: &Options) {
    if options.format != "text" || options.profile.is_some() {
        eprintln!("unsupported option for this command");
        process::exit(1);
    }
}

fn reject_profile(options: &Options) {
    if options.profile.is_some() {
        eprintln!("--profile is only supported by validate");
        process::exit(1);
    }
}

fn print_diagnostics(diagnostics: &[nodx_core::Diagnostic]) {
    for d in diagnostics {
        let loc = match (d.line, d.column) {
            (Some(l), Some(c)) => format!("{}:{} ", l, c),
            _ => String::new(),
        };
        println!("{} {}{} {}", d.severity, loc, d.code, d.message);
    }
}

fn parse_or_exit(bytes: &[u8]) -> nodx_core::Document {
    parse_bytes(bytes).unwrap_or_else(|diag| {
        eprintln!("{}: {}", diag.code, diag.message);
        process::exit(2);
    })
}

fn diagnostic_exit_code(doc: &nodx_core::Document) -> i32 {
    if doc
        .diagnostics
        .iter()
        .any(|d| d.severity == "fatal" || d.severity == "error")
    {
        2
    } else {
        0
    }
}
