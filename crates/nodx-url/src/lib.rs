#![forbid(unsafe_code)]

pub use nodx_core::ResourceLimits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceKind {
    Link,
    Asset,
    Style,
    Include,
    Font,
    MediaFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UriClass {
    Absolute { scheme: String },
    Data { mime: String },
    Fragment,
    PackageRelative { path: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassifiedUri {
    pub raw: String,
    pub class: UriClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UrlError {
    Empty,
    TooLong,
    ControlCharacter,
    Backslash,
    UnsafeScheme,
    UnsafeDataUri,
    PathTraversal,
    AbsolutePath,
    InvalidPackagePath,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourcePolicy {
    limits: ResourceLimits,
}

impl ResourcePolicy {
    pub fn new(limits: ResourceLimits) -> Self {
        Self { limits }
    }

    pub fn limits(&self) -> ResourceLimits {
        self.limits
    }

    pub fn classify_uri(&self, kind: ReferenceKind, raw: &str) -> Result<ClassifiedUri, UrlError> {
        let trimmed = raw.trim_matches(|c: char| c.is_ascii_whitespace());
        if trimmed.is_empty() {
            return Err(UrlError::Empty);
        }
        if trimmed.len() > self.limits.url_bytes {
            return Err(UrlError::TooLong);
        }
        reject_control_or_backslash(trimmed)?;

        if trimmed.starts_with('#') {
            return match kind {
                ReferenceKind::Link => Ok(ClassifiedUri {
                    raw: trimmed.to_string(),
                    class: UriClass::Fragment,
                }),
                _ => Err(UrlError::InvalidPackagePath),
            };
        }

        if let Some((scheme, _colon)) = scheme_prefix(trimmed)? {
            return self.classify_scheme(kind, trimmed, &scheme);
        }

        let path = normalize_package_path(trimmed, self.limits)?;
        Ok(ClassifiedUri {
            raw: path.clone(),
            class: UriClass::PackageRelative { path },
        })
    }

    pub fn normalize_package_path(&self, raw: &str) -> Result<String, UrlError> {
        normalize_package_path(raw, self.limits)
    }

    fn classify_scheme(
        &self,
        kind: ReferenceKind,
        raw: &str,
        scheme: &str,
    ) -> Result<ClassifiedUri, UrlError> {
        if is_forbidden_scheme(scheme) {
            return Err(UrlError::UnsafeScheme);
        }
        match kind {
            ReferenceKind::Link if matches!(scheme, "http" | "https" | "mailto" | "tel") => {
                Ok(ClassifiedUri {
                    raw: raw.to_string(),
                    class: UriClass::Absolute {
                        scheme: scheme.to_string(),
                    },
                })
            }
            ReferenceKind::Asset if scheme == "data" => {
                let mime = validate_data_uri(raw, self.limits)?;
                Ok(ClassifiedUri {
                    raw: raw.to_string(),
                    class: UriClass::Data { mime },
                })
            }
            _ => Err(UrlError::UnsafeScheme),
        }
    }
}

pub fn classify_uri(kind: ReferenceKind, raw: &str) -> Result<ClassifiedUri, UrlError> {
    ResourcePolicy::default().classify_uri(kind, raw)
}

pub fn normalize_package_path(raw: &str, limits: ResourceLimits) -> Result<String, UrlError> {
    let trimmed = raw.trim_matches(|c: char| c.is_ascii_whitespace());
    if trimmed.is_empty() {
        return Err(UrlError::Empty);
    }
    if trimmed.len() > limits.url_bytes {
        return Err(UrlError::TooLong);
    }
    reject_control_or_backslash(trimmed)?;
    if trimmed.starts_with('/') {
        return Err(UrlError::AbsolutePath);
    }
    if scheme_prefix(trimmed)?.is_some() {
        return Err(UrlError::UnsafeScheme);
    }
    if trimmed.len() > limits.package_path_bytes {
        return Err(UrlError::InvalidPackagePath);
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() > limits.package_path_segments {
        return Err(UrlError::InvalidPackagePath);
    }

    let mut normalized = Vec::with_capacity(parts.len());
    for part in &parts {
        if part.is_empty() || *part == "." {
            return Err(UrlError::InvalidPackagePath);
        }
        let decoded = percent_decode_ascii(part)?;
        if *part == ".." || decoded == ".." {
            return Err(UrlError::PathTraversal);
        }
        if decoded.contains(':')
            || decoded.contains('/')
            || decoded.contains('\\')
            || decoded
                .chars()
                .any(|c| (c as u32) < 0x20 || c == '\u{007f}')
        {
            return Err(UrlError::InvalidPackagePath);
        }
        normalized.push(*part);
    }
    Ok(normalized.join("/"))
}

fn reject_control_or_backslash(input: &str) -> Result<(), UrlError> {
    if input.contains('\\') {
        return Err(UrlError::Backslash);
    }
    if input.chars().any(|c| (c as u32) < 0x20 || c == '\u{007f}') {
        return Err(UrlError::ControlCharacter);
    }
    Ok(())
}

fn scheme_prefix(input: &str) -> Result<Option<(String, usize)>, UrlError> {
    let boundary = input.find([':', '/', '?', '#']).unwrap_or(input.len());
    if input.as_bytes().get(boundary) != Some(&b':') {
        return Ok(None);
    }
    if boundary == 0 {
        return Err(UrlError::UnsafeScheme);
    }
    let decoded = percent_decode_ascii(&input[..boundary])?.to_ascii_lowercase();
    if decoded.is_empty() || !decoded.chars().all(is_scheme_char) {
        return Err(UrlError::UnsafeScheme);
    }
    Ok(Some((decoded, boundary)))
}

fn validate_data_uri(raw: &str, limits: ResourceLimits) -> Result<String, UrlError> {
    if raw.len() > limits.data_uri_bytes {
        return Err(UrlError::TooLong);
    }
    let colon = raw.find(':').ok_or(UrlError::UnsafeDataUri)?;
    if !raw[..colon].eq_ignore_ascii_case("data") {
        return Err(UrlError::UnsafeDataUri);
    }
    let rest = &raw[colon + 1..];
    let comma = rest.find(',').ok_or(UrlError::UnsafeDataUri)?;
    let meta = &rest[..comma];
    let mime = meta
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if matches!(
        mime.as_str(),
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        Ok(mime)
    } else {
        Err(UrlError::UnsafeDataUri)
    }
}

fn is_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')
}

fn is_forbidden_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "javascript" | "vbscript" | "file" | "jar" | "chrome" | "about"
    )
}

fn percent_decode_ascii(input: &str) -> Result<String, UrlError> {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(UrlError::UnsafeScheme);
            }
            let hi = hex_value(bytes[i + 1]).ok_or(UrlError::UnsafeScheme)?;
            let lo = hex_value(bytes[i + 2]).ok_or(UrlError::UnsafeScheme)?;
            let v = (hi << 4) | lo;
            if v >= 0x80 {
                return Err(UrlError::UnsafeScheme);
            }
            out.push(v as char);
            i += 3;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    Ok(out)
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_security_corpus_matches_policy() {
        let corpus = include_str!("../../../spec/tests/security/url-policy.tsv");
        let policy = ResourcePolicy::default();
        for (line_number, line) in corpus.lines().enumerate() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 3, "line {}", line_number + 1);
            let kind = match fields[0] {
                "link" => ReferenceKind::Link,
                "asset" => ReferenceKind::Asset,
                "style" => ReferenceKind::Style,
                "include" => ReferenceKind::Include,
                "font" => ReferenceKind::Font,
                "media-fallback" => ReferenceKind::MediaFallback,
                other => panic!("unknown kind `{other}` on line {}", line_number + 1),
            };
            let allowed = fields[1] == "allow";
            let result = policy.classify_uri(kind, fields[2]);
            assert_eq!(
                result.is_ok(),
                allowed,
                "line {}: {:?}",
                line_number + 1,
                result
            );
        }
    }

    #[test]
    fn link_policy_allows_expected_references() {
        let policy = ResourcePolicy::default();
        assert!(
            policy
                .classify_uri(ReferenceKind::Link, "https://example.test")
                .is_ok()
        );
        assert!(
            policy
                .classify_uri(ReferenceKind::Link, "mailto:a@example.test")
                .is_ok()
        );
        assert!(
            policy
                .classify_uri(ReferenceKind::Link, "tel:+15551212")
                .is_ok()
        );
        assert!(policy.classify_uri(ReferenceKind::Link, "#section").is_ok());
        assert!(
            policy
                .classify_uri(ReferenceKind::Link, "docs/page.nodx")
                .is_ok()
        );
    }

    #[test]
    fn dangerous_schemes_and_obfuscation_are_rejected() {
        let policy = ResourcePolicy::default();
        for raw in [
            "javascript:alert(1)",
            " javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "ja%76ascript:alert(1)",
            "javascript%3aalert(1)",
            "vbscript:alert(1)",
            "file:///etc/passwd",
            "jar:https://example.test/x.jar!/",
            "chrome://settings",
            "about:blank",
        ] {
            assert!(
                policy.classify_uri(ReferenceKind::Link, raw).is_err(),
                "{raw}"
            );
        }
    }

    #[test]
    fn control_backslash_and_traversal_are_rejected() {
        let policy = ResourcePolicy::default();
        for raw in [
            "\u{0008}javascript:alert",
            "dir\\file.png",
            "../secret.png",
            "dir/../secret.png",
            "dir/%2e%2e/secret.png",
            "dir/%2f/secret.png",
            "/absolute/path.png",
            "dir//file.png",
            "dir/./file.png",
        ] {
            assert!(
                policy.classify_uri(ReferenceKind::Asset, raw).is_err(),
                "{raw}"
            );
        }
    }

    #[test]
    fn assets_allow_package_relative_and_safe_data_only() {
        let policy = ResourcePolicy::default();
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "images/a.png")
                .is_ok()
        );
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "data:image/png;base64,AAA")
                .is_ok()
        );
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "data:image/svg+xml,<svg/>")
                .is_err()
        );
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "data:text/html,<script>")
                .is_err()
        );
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "https://example.test/x.png")
                .is_err()
        );
    }

    #[test]
    fn data_uri_size_limit_is_enforced() {
        let policy = ResourcePolicy::new(ResourceLimits {
            data_uri_bytes: 20,
            ..ResourceLimits::default()
        });
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "data:image/png;base64,AAA")
                .is_err()
        );
    }

    #[test]
    fn non_link_kinds_are_package_relative_only() {
        let policy = ResourcePolicy::default();
        for kind in [
            ReferenceKind::Style,
            ReferenceKind::Include,
            ReferenceKind::Font,
            ReferenceKind::MediaFallback,
        ] {
            assert!(policy.classify_uri(kind, "assets/file.bin").is_ok());
            assert!(
                policy
                    .classify_uri(kind, "https://example.test/file.bin")
                    .is_err()
            );
            assert!(
                policy
                    .classify_uri(kind, "data:image/png;base64,AAA")
                    .is_err()
            );
            assert!(policy.classify_uri(kind, "#frag").is_err());
        }
    }

    #[test]
    fn package_path_limits_are_enforced() {
        let policy = ResourcePolicy::new(ResourceLimits {
            package_path_bytes: 20,
            package_path_segments: 3,
            ..ResourceLimits::default()
        });
        assert!(
            policy
                .classify_uri(ReferenceKind::Asset, "a/b/c/d.png")
                .is_err()
        );
        let long = format!("{}/x.png", "a".repeat(30));
        assert!(policy.classify_uri(ReferenceKind::Asset, &long).is_err());
    }
}
