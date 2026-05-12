use std::{env, fs, path::PathBuf, process};

use nodx_core::{
    ResourceLimits, canonical_json, is_packaged_nodx, parse_bytes_with_limits, render_tui,
};
use nodx_export::{ExportFormat, export_document, loss_report_json};
use nodx_ncp::ncp_json;
use nodx_package::{Package, apply_package_extensions};
use nodx_render_html::{RenderOptions, render_html_with_options};
use nodx_validate::{ProfileSet, Validator, diagnostics_json, exit_code_for};

const USAGE: &str = concat!(
    "nodx 1.0\n",
    "\n",
    "Commands:\n",
    "  nodx inspect <file>\n",
    "  nodx ast <file> [--format text|json]\n",
    "  nodx validate <file> [--profile plain|core|rich|style|package|agent-read] [--format text|json]\n",
    "  nodx diagnostics <file> [--format text|json]\n",
    "  nodx html <file> [--standalone|--fragment] [--csp|--no-csp]\n",
    "  nodx tui <file>\n",
    "  nodx ncp <file> [--mode semantic]\n",
    "  nodx package inspect <file>\n",
    "  nodx package verify <file>\n",
    "  nodx export pdf|docx|pptx <file> -o <out>   (unstable preview)\n",
);

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("{USAGE}");
        process::exit(1);
    }
    let command = args[1].as_str();
    let exit = match command {
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            0
        }
        "package" => package_command(&args[2..]),
        "export" => export_command(&args[2..]),
        _ => document_command(command, &args[2..]),
    };
    if exit != 0 {
        process::exit(exit);
    }
}

fn document_command(command: &str, rest: &[String]) -> i32 {
    if rest.is_empty() {
        eprintln!("{USAGE}");
        return 1;
    }
    let file = rest[0].as_str();
    let options = match parse_options(&rest[1..]) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let bytes = match fs::read(file) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("read error: {err}");
            return 1;
        }
    };
    match command {
        "inspect" => {
            reject_irrelevant(&options, &["format"]);
            print_inspect(&bytes);
            0
        }
        "ast" => {
            reject_irrelevant(&options, &["format"]);
            let doc = match load_document(&bytes) {
                Ok(doc) => doc,
                Err(code) => return code,
            };
            if options.format == "json" || options.format == "text" {
                println!("{}", canonical_json(&doc));
            } else {
                eprintln!("--format must be text or json");
                return 1;
            }
            diagnostic_exit_code(&doc)
        }
        "html" => {
            reject_irrelevant(&options, &["standalone", "csp"]);
            let doc = match load_document(&bytes) {
                Ok(doc) => doc,
                Err(code) => return code,
            };
            let render_options = RenderOptions {
                standalone: options.standalone.unwrap_or(true),
                include_csp: options.include_csp.unwrap_or(true),
                limits: ResourceLimits::default(),
            };
            println!("{}", render_html_with_options(&doc, render_options));
            diagnostic_exit_code(&doc)
        }
        "tui" => {
            reject_irrelevant(&options, &[]);
            let doc = match load_document(&bytes) {
                Ok(doc) => doc,
                Err(code) => return code,
            };
            print!("{}", render_tui(&doc));
            diagnostic_exit_code(&doc)
        }
        "ncp" => {
            reject_irrelevant(&options, &["mode"]);
            let doc = match load_document(&bytes) {
                Ok(doc) => doc,
                Err(code) => return code,
            };
            match options.mode.as_deref().unwrap_or("semantic") {
                "semantic" => println!("{}", ncp_json(&doc)),
                other => {
                    eprintln!("unsupported NCP mode: {other}");
                    return 1;
                }
            }
            diagnostic_exit_code(&doc)
        }
        "diagnostics" => {
            reject_irrelevant(&options, &["format"]);
            let doc = match load_document(&bytes) {
                Ok(doc) => doc,
                Err(code) => return code,
            };
            let diagnostics = Validator::default().validate(&doc);
            print_diagnostics_set(&diagnostics, &options);
            exit_code_for(&diagnostics)
        }
        "validate" => {
            reject_irrelevant(&options, &["format", "profile"]);
            let doc = match load_document(&bytes) {
                Ok(doc) => doc,
                Err(code) => return code,
            };
            let validator = Validator::default();
            let diagnostics = validator.validate_with_profile(&doc, options.profile.as_deref());
            print_diagnostics_set(&diagnostics, &options);
            exit_code_for(&diagnostics)
        }
        other => {
            eprintln!("unknown command: {other}");
            1
        }
    }
}

fn load_document(bytes: &[u8]) -> Result<nodx_core::Document, i32> {
    let limits = ResourceLimits::default();
    if is_packaged_nodx(bytes) {
        let package = match Package::open(bytes, limits) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("{}: {}", err.code, err.message);
                return Err(2);
            }
        };
        match parse_bytes_with_limits(package.entry_bytes(), limits) {
            Ok(doc) => apply_package_extensions(&doc, &package).map_err(|err| {
                eprintln!("{}: {}", err.code, err.message);
                2
            }),
            Err(diag) => {
                eprintln!("{}: {}", diag.code, diag.message);
                Err(if diag.code == "NODX-E024" { 3 } else { 2 })
            }
        }
    } else {
        match parse_bytes_with_limits(bytes, limits) {
            Ok(doc) => Ok(doc),
            Err(diag) => {
                eprintln!("{}: {}", diag.code, diag.message);
                Err(if diag.code == "NODX-E024" { 3 } else { 2 })
            }
        }
    }
}

fn package_command(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("usage: nodx package <inspect|verify> <file>");
        return 1;
    }
    let sub = args[0].as_str();
    let file = args[1].as_str();
    let bytes = match fs::read(file) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("read error: {err}");
            return 1;
        }
    };
    if !is_packaged_nodx(&bytes) {
        eprintln!("{file}: not a packaged NODX (missing PK\\x03\\x04 signature)");
        return 2;
    }
    let limits = ResourceLimits::default();
    let package = match Package::open(&bytes, limits) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("{}: {}", err.code, err.message);
            return 2;
        }
    };
    match sub {
        "inspect" => {
            println!("format: packaged-nodx");
            println!("container: zip");
            println!("entry: {}", package.entry_path());
            if let Some(sig) = package.signature_path() {
                println!("signature: {sig}");
            } else {
                println!("signature: (none)");
            }
            for profile in package.profiles_required() {
                println!("profile-required: {profile}");
            }
            for profile in package.profiles_optional() {
                println!("profile-optional: {profile}");
            }
            for path in package.fs().paths() {
                println!("entry-path: {path}");
            }
            0
        }
        "verify" => {
            let validator = Validator::default();
            let doc = match parse_bytes_with_limits(package.entry_bytes(), limits) {
                Ok(doc) => doc,
                Err(diag) => {
                    eprintln!("{}: {}", diag.code, diag.message);
                    return 2;
                }
            };
            let diagnostics = validator.validate(&doc);
            for profile in package.profiles_required() {
                if !ProfileSet::default().supports(profile) {
                    eprintln!("NODX-E024: required package profile `{profile}` is unsupported.");
                    return 3;
                }
            }
            println!(
                "digest: {}",
                nodx_core::sha256_base64url(package.entry_bytes())
            );
            println!("entries: {}", package.fs().paths().count());
            println!("valid: {}", exit_code_for(&diagnostics) == 0);
            if !diagnostics.is_empty() {
                print_diagnostics(&diagnostics);
            }
            exit_code_for(&diagnostics)
        }
        other => {
            eprintln!("unknown package subcommand: {other}");
            1
        }
    }
}

fn export_command(args: &[String]) -> i32 {
    eprintln!(
        "warning: `nodx export` is an unstable preview of the deferred export profile; PDF is a paged HTML host bridge and DOCX/PPTX emit loss reports."
    );
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
    let doc = match load_document(&bytes) {
        Ok(doc) => doc,
        Err(code) => return code,
    };
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

#[derive(Clone, Debug, Default)]
struct Options {
    format: String,
    profile: Option<String>,
    mode: Option<String>,
    standalone: Option<bool>,
    include_csp: Option<bool>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        format: "text".to_string(),
        ..Options::default()
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
            "--mode" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--mode requires a value".to_string())?;
                options.mode = Some(value.clone());
                i += 2;
            }
            "--standalone" => {
                options.standalone = Some(true);
                i += 1;
            }
            "--fragment" => {
                options.standalone = Some(false);
                i += 1;
            }
            "--csp" => {
                options.include_csp = Some(true);
                i += 1;
            }
            "--no-csp" => {
                options.include_csp = Some(false);
                i += 1;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(options)
}

fn reject_irrelevant(options: &Options, allowed: &[&str]) {
    let allowed: std::collections::BTreeSet<&str> = allowed.iter().copied().collect();
    let mut bad = Vec::new();
    if options.format != "text" && !allowed.contains("format") {
        bad.push("--format");
    }
    if options.profile.is_some() && !allowed.contains("profile") {
        bad.push("--profile");
    }
    if options.mode.is_some() && !allowed.contains("mode") {
        bad.push("--mode");
    }
    if options.standalone.is_some() && !allowed.contains("standalone") {
        bad.push("--standalone/--fragment");
    }
    if options.include_csp.is_some() && !allowed.contains("csp") {
        bad.push("--csp/--no-csp");
    }
    if !bad.is_empty() {
        eprintln!("unsupported option for this command: {}", bad.join(", "));
        process::exit(1);
    }
}

fn print_diagnostics_set(diagnostics: &[nodx_core::Diagnostic], options: &Options) {
    if options.format == "json" {
        println!("{}", diagnostics_json(diagnostics));
    } else {
        print_diagnostics(diagnostics);
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

fn print_inspect(bytes: &[u8]) {
    if is_packaged_nodx(bytes) {
        println!("format: packaged-nodx");
        println!("container: zip");
    } else {
        println!("format: text-nodx");
        println!("container: utf-8");
    }
}

fn diagnostic_exit_code(doc: &nodx_core::Document) -> i32 {
    exit_code_for(&doc.diagnostics)
}
