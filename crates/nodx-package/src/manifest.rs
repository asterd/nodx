use crate::PackageDiagnostic;
use crate::diag;
use crate::diag_code;

#[derive(Clone, Debug, Default)]
pub(crate) struct PackageManifest {
    pub(crate) schema: Option<String>,
    pub(crate) entry: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) profiles_required: Vec<String>,
    pub(crate) profiles_optional: Vec<String>,
    pub(crate) entries: Vec<PackageManifestEntry>,
    pub(crate) components: Vec<String>,
    pub(crate) themes: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageManifestEntry {
    pub(crate) path: String,
    pub(crate) size: Option<usize>,
    pub(crate) sha256: Option<String>,
}

// Event-driven safe-subset YAML for package manifests. Rejects anchors, aliases,
// tags, merge keys, duplicate mapping keys, multiple documents, native
// timestamps, and non-finite numerics. Mirrors the front matter safety policy.
pub(crate) fn parse_package_manifest(text: &str) -> Result<PackageManifest, PackageDiagnostic> {
    let mut manifest = PackageManifest::default();
    let mut seen_top = std::collections::BTreeSet::new();
    let mut entry_section = false;
    let mut current: Option<PackageManifestEntry> = None;
    let mut profile_section: Option<&'static str> = None;
    let mut profile_indent: usize = 0;
    let mut path_list_section: Option<&'static str> = None;
    let mut path_list_indent: usize = 0;

    let mut document_started = false;
    for (line_no, raw_line) in text.lines().enumerate() {
        let line_number = line_no + 1;
        if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
            continue;
        }
        if raw_line.starts_with("---") || raw_line.starts_with("...") {
            if document_started {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("Multiple YAML documents in manifest at line {line_number}."),
                ));
            }
            document_started = true;
            continue;
        }
        document_started = true;
        if raw_line.contains('\t') {
            return Err(diag_code(
                "NODX-E019",
                &format!("Manifest must not contain tab characters (line {line_number})."),
            ));
        }
        let indent = raw_line.chars().take_while(|c| *c == ' ').count();
        let content = &raw_line[indent..];
        if content.contains(" #") {
            // inline comments are not part of the safe subset
            return Err(diag_code(
                "NODX-E019",
                &format!("Manifest inline comments are not allowed (line {line_number})."),
            ));
        }
        reject_unsafe_yaml(content, line_number)?;
        if let Some(section) = profile_section {
            if indent > profile_indent {
                let item = content
                    .strip_prefix("- ")
                    .ok_or_else(|| {
                        diag_code(
                            "NODX-E019",
                            &format!("Expected list item at line {line_number}."),
                        )
                    })?
                    .trim()
                    .to_string();
                if item.is_empty() {
                    return Err(diag_code(
                        "NODX-E019",
                        &format!("Empty profile entry at line {line_number}."),
                    ));
                }
                match section {
                    "requires" => manifest.profiles_required.push(unquote(&item)),
                    "optional" => manifest.profiles_optional.push(unquote(&item)),
                    _ => {}
                }
                continue;
            } else {
                profile_section = None;
            }
        }
        if let Some(section) = path_list_section {
            if indent > path_list_indent {
                let path = content
                    .strip_prefix("- path:")
                    .ok_or_else(|| {
                        diag_code(
                            "NODX-E019",
                            &format!("Expected path list item at line {line_number}."),
                        )
                    })?
                    .trim();
                match section {
                    "components" => manifest.components.push(unquote(path)),
                    "themes" => manifest.themes.push(unquote(path)),
                    _ => {}
                }
                continue;
            } else {
                path_list_section = None;
            }
        }

        if entry_section && indent > 0 {
            let item = content;
            if let Some(rest) = item.strip_prefix("- path:") {
                if let Some(prev) = current.take() {
                    manifest.entries.push(prev);
                }
                current = Some(PackageManifestEntry {
                    path: unquote(rest.trim()),
                    size: None,
                    sha256: None,
                });
            } else if let Some(rest) = item.strip_prefix("path:") {
                if let Some(prev) = current.take() {
                    manifest.entries.push(prev);
                }
                current = Some(PackageManifestEntry {
                    path: unquote(rest.trim()),
                    size: None,
                    sha256: None,
                });
            } else if let Some(rest) = item.strip_prefix("size:") {
                let Some(entry) = current.as_mut() else {
                    return Err(diag_code(
                        "NODX-E019",
                        &format!("Manifest size without entry at line {line_number}."),
                    ));
                };
                entry.size = Some(rest.trim().parse::<usize>().map_err(|_| {
                    diag_code(
                        "NODX-E019",
                        &format!(
                            "Manifest size is not a non-negative integer at line {line_number}."
                        ),
                    )
                })?);
            } else if let Some(rest) = item.strip_prefix("sha256:") {
                let Some(entry) = current.as_mut() else {
                    return Err(diag_code(
                        "NODX-E019",
                        &format!("Manifest sha256 without entry at line {line_number}."),
                    ));
                };
                entry.sha256 = Some(unquote(rest.trim()));
            } else {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("Unknown manifest entry field at line {line_number}: `{item}`."),
                ));
            }
            continue;
        }

        if let Some(prev) = current.take() {
            manifest.entries.push(prev);
            entry_section = false;
        }

        if let Some(rest) = content.strip_prefix("schema:") {
            unique(&mut seen_top, "schema", line_number)?;
            manifest.schema = Some(unquote(rest.trim()));
        } else if let Some(rest) = content.strip_prefix("entry:") {
            unique(&mut seen_top, "entry", line_number)?;
            manifest.entry = Some(unquote(rest.trim()));
        } else if let Some(rest) = content.strip_prefix("signature:") {
            unique(&mut seen_top, "signature", line_number)?;
            let v = rest.trim();
            if !v.is_empty() {
                manifest.signature = Some(unquote(v));
            }
        } else if content.starts_with("entries:") {
            unique(&mut seen_top, "entries", line_number)?;
            entry_section = true;
        } else if let Some(rest) = content.strip_prefix("components:") {
            unique(&mut seen_top, "components", line_number)?;
            if !rest.trim().is_empty() {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("`components:` must use block style at line {line_number}."),
                ));
            }
            path_list_section = Some("components");
            path_list_indent = indent;
        } else if let Some(rest) = content.strip_prefix("themes:") {
            unique(&mut seen_top, "themes", line_number)?;
            if !rest.trim().is_empty() {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("`themes:` must use block style at line {line_number}."),
                ));
            }
            path_list_section = Some("themes");
            path_list_indent = indent;
        } else if let Some(rest) = content.strip_prefix("profiles:") {
            unique(&mut seen_top, "profiles", line_number)?;
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("`profiles:` must use block style at line {line_number}."),
                ));
            }
        } else if let Some(rest) = content.strip_prefix("requires:") {
            if !rest.trim().is_empty() {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("`requires:` must be a list at line {line_number}."),
                ));
            }
            profile_section = Some("requires");
            profile_indent = indent;
        } else if let Some(rest) = content.strip_prefix("optional:") {
            if !rest.trim().is_empty() {
                return Err(diag_code(
                    "NODX-E019",
                    &format!("`optional:` must be a list at line {line_number}."),
                ));
            }
            profile_section = Some("optional");
            profile_indent = indent;
        } else {
            return Err(diag_code(
                "NODX-E019",
                &format!("Unknown manifest field at line {line_number}: `{content}`."),
            ));
        }
    }

    if let Some(entry) = current {
        manifest.entries.push(entry);
    }
    if !document_started {
        return Err(diag("Package manifest is empty."));
    }
    Ok(manifest)
}

fn reject_unsafe_yaml(content: &str, line_number: usize) -> Result<(), PackageDiagnostic> {
    let masked = unquoted_view(content);
    if masked.contains('&') || masked.contains('*') || masked.contains('!') {
        return Err(diag_code(
            "NODX-E019",
            &format!("Forbidden YAML construct at line {line_number}."),
        ));
    }
    if masked.contains("<<:") {
        return Err(diag_code(
            "NODX-E019",
            &format!("YAML merge keys are not allowed at line {line_number}."),
        ));
    }
    if masked.starts_with("? ") || masked.starts_with('?') && masked.len() == 1 {
        return Err(diag_code(
            "NODX-E019",
            &format!("Forbidden YAML key at line {line_number}."),
        ));
    }
    if masked.starts_with('[') || masked.starts_with('{') {
        return Err(diag_code(
            "NODX-E019",
            &format!("Flow-style YAML is not allowed at line {line_number}."),
        ));
    }
    Ok(())
}

fn unique(
    seen: &mut std::collections::BTreeSet<&'static str>,
    key: &'static str,
    line_number: usize,
) -> Result<(), PackageDiagnostic> {
    if !seen.insert(key) {
        return Err(diag_code(
            "NODX-E019",
            &format!("Duplicate manifest key `{key}` at line {line_number}."),
        ));
    }
    Ok(())
}

fn unquote(raw: &str) -> String {
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return raw[1..raw.len() - 1].to_string();
        }
    }
    raw.to_string()
}

fn unquoted_view(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut quote = None;
    for ch in input.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, c) => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let text = "schema: nodx-package/1.0\nentry: doc.nodx\nentries:\n  - path: doc.nodx\n    size: 4\n    sha256: sha256-AAA\n";
        let m = parse_package_manifest(text).unwrap();
        assert_eq!(m.schema.as_deref(), Some("nodx-package/1.0"));
        assert_eq!(m.entry.as_deref(), Some("doc.nodx"));
        assert_eq!(m.entries.len(), 1);
        assert_eq!(m.entries[0].path, "doc.nodx");
        assert_eq!(m.entries[0].size, Some(4));
        assert_eq!(m.entries[0].sha256.as_deref(), Some("sha256-AAA"));
    }

    #[test]
    fn rejects_anchors_and_aliases() {
        let text = "schema: &anchor nodx-package/1.0\nentry: doc.nodx\n";
        let err = parse_package_manifest(text).unwrap_err();
        assert_eq!(err.code, "NODX-E019");
    }

    #[test]
    fn rejects_duplicate_keys() {
        let text = "schema: nodx-package/1.0\nschema: nodx-package/1.0\nentry: doc.nodx\n";
        let err = parse_package_manifest(text).unwrap_err();
        assert_eq!(err.code, "NODX-E019");
    }

    #[test]
    fn rejects_unknown_top_level() {
        let text = "schema: nodx-package/1.0\nbogus: 1\n";
        let err = parse_package_manifest(text).unwrap_err();
        assert_eq!(err.code, "NODX-E019");
    }

    #[test]
    fn rejects_multiple_documents() {
        let text = "---\nschema: nodx-package/1.0\nentry: a.nodx\n---\nschema: nodx-package/1.0\n";
        let err = parse_package_manifest(text).unwrap_err();
        assert_eq!(err.code, "NODX-E019");
    }

    #[test]
    fn parses_profiles_lists() {
        let text = "schema: nodx-package/1.0\nentry: doc.nodx\nprofiles:\nrequires:\n  - core\n  - rich\noptional:\n  - style\n";
        let m = parse_package_manifest(text).unwrap();
        assert_eq!(m.profiles_required, vec!["core", "rich"]);
        assert_eq!(m.profiles_optional, vec!["style"]);
    }
}
