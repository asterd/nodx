use std::{env, fs, process};

use nodx_core::{canonical_json, is_packaged_nodx, ncp_json, parse_bytes, render_html, render_tui};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: nodx <ast|html|tui|ncp|diagnostics|validate|inspect> <file.nodx>");
        process::exit(2);
    }
    let bytes = fs::read(&args[2]).unwrap_or_else(|err| {
        eprintln!("read error: {err}");
        process::exit(1);
    });
    let exit = match args[1].as_str() {
        "inspect" => {
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
            let doc = parse_or_exit(&bytes);
            println!("{}", canonical_json(&doc));
            fatal_exit_code(&doc)
        }
        "html" => {
            let doc = parse_or_exit(&bytes);
            println!("{}", render_html(&doc));
            fatal_exit_code(&doc)
        }
        "tui" => {
            let doc = parse_or_exit(&bytes);
            print!("{}", render_tui(&doc));
            fatal_exit_code(&doc)
        }
        "ncp" => {
            let doc = parse_or_exit(&bytes);
            println!("{}", ncp_json(&doc));
            fatal_exit_code(&doc)
        }
        "diagnostics" => {
            let doc = parse_or_exit(&bytes);
            for d in &doc.diagnostics {
                let loc = match (d.line, d.column) {
                    (Some(l), Some(c)) => format!("{}:{} ", l, c),
                    _ => String::new(),
                };
                println!("{} {}{} {}", d.severity, loc, d.code, d.message);
            }
            fatal_exit_code(&doc)
        }
        "validate" => {
            let doc = parse_or_exit(&bytes);
            for d in &doc.diagnostics {
                let loc = match (d.line, d.column) {
                    (Some(l), Some(c)) => format!("{}:{} ", l, c),
                    _ => String::new(),
                };
                println!("{} {}{} {}", d.severity, loc, d.code, d.message);
            }
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
        other => {
            eprintln!("unknown command: {other}");
            2
        }
    };
    if exit != 0 {
        process::exit(exit);
    }
}

fn parse_or_exit(bytes: &[u8]) -> nodx_core::Document {
    parse_bytes(bytes).unwrap_or_else(|diag| {
        eprintln!("{}: {}", diag.code, diag.message);
        process::exit(2);
    })
}

fn fatal_exit_code(doc: &nodx_core::Document) -> i32 {
    if doc.diagnostics.iter().any(|d| d.severity == "fatal") {
        2
    } else {
        0
    }
}
