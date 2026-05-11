#![forbid(unsafe_code)]

use nodx_core::{Document, canonical_json, parse_bytes};
use nodx_package::Package;
use nodx_url::ResourceLimits;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JwsHeader {
    pub alg: String,
    pub kid: Option<String>,
    pub typ: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CryptographicStatus {
    Valid,
    Invalid,
    NotChecked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustStatus {
    Trusted,
    Untrusted,
}

#[derive(Clone, Debug)]
pub struct Es256PublicKey {
    key: VerifyingKey,
}

#[derive(Clone, Debug)]
pub struct TrustDecision {
    pub status: TrustStatus,
    pub es256_key: Option<Es256PublicKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    pub digest: String,
    pub header: JwsHeader,
    pub cryptographic_status: CryptographicStatus,
    pub trust_status: TrustStatus,
}

pub trait TrustPolicy {
    fn evaluate(&self, header: &JwsHeader) -> TrustDecision;
}

#[derive(Clone, Debug)]
pub struct StaticTrustPolicy {
    kid: Option<String>,
    key: Es256PublicKey,
    status: TrustStatus,
}

impl Es256PublicKey {
    pub fn from_sec1_bytes(input: &[u8]) -> Result<Self, SignDiagnostic> {
        let key = VerifyingKey::from_sec1_bytes(input)
            .map_err(|_| diag("Invalid ES256 public key bytes."))?;
        Ok(Self { key })
    }
}

impl TrustDecision {
    pub fn trusted_es256(key: Es256PublicKey) -> Self {
        Self {
            status: TrustStatus::Trusted,
            es256_key: Some(key),
        }
    }

    pub fn untrusted_es256(key: Es256PublicKey) -> Self {
        Self {
            status: TrustStatus::Untrusted,
            es256_key: Some(key),
        }
    }

    pub fn untrusted_without_key() -> Self {
        Self {
            status: TrustStatus::Untrusted,
            es256_key: None,
        }
    }
}

impl StaticTrustPolicy {
    pub fn trusted(kid: Option<&str>, key: Es256PublicKey) -> Self {
        Self {
            kid: kid.map(str::to_string),
            key,
            status: TrustStatus::Trusted,
        }
    }

    pub fn untrusted(kid: Option<&str>, key: Es256PublicKey) -> Self {
        Self {
            kid: kid.map(str::to_string),
            key,
            status: TrustStatus::Untrusted,
        }
    }
}

impl TrustPolicy for StaticTrustPolicy {
    fn evaluate(&self, header: &JwsHeader) -> TrustDecision {
        if self.kid.as_deref().is_some() && self.kid != header.kid {
            return TrustDecision::untrusted_without_key();
        }
        TrustDecision {
            status: self.status.clone(),
            es256_key: Some(self.key.clone()),
        }
    }
}

pub fn canonical_digest(doc: &Document) -> String {
    nodx_package::sha256_base64url(canonical_json(doc).as_bytes())
}

pub fn digest_text_nodx(input: &[u8]) -> Result<String, SignDiagnostic> {
    let doc = parse_bytes(input).map_err(|err| diag(&err.message))?;
    Ok(canonical_digest(&doc))
}

pub fn verify_detached_jws(
    text_nodx: &[u8],
    compact_jws: &str,
    trust_policy: &dyn TrustPolicy,
) -> Result<VerificationResult, SignDiagnostic> {
    let digest = digest_text_nodx(text_nodx)?;
    verify_compact_jws_for_digest(&digest, compact_jws, true, trust_policy)
}

pub fn verify_packaged_signature(
    package_bytes: &[u8],
    signature_path: &str,
    trust_policy: &dyn TrustPolicy,
) -> Result<VerificationResult, SignDiagnostic> {
    let package = Package::open(package_bytes, ResourceLimits::default())
        .map_err(|err| diag(&err.message))?;
    let digest = digest_text_nodx(package.entry_bytes())?;
    let signature = package
        .fs()
        .read(signature_path)
        .ok_or_else(|| diag("Package signature entry is missing."))?;
    let signature_text =
        std::str::from_utf8(signature).map_err(|_| diag("Package signature is not UTF-8."))?;
    verify_compact_jws_for_digest(&digest, signature_text.trim(), false, trust_policy)
}

pub fn verify_compact_jws_for_digest(
    digest: &str,
    compact_jws: &str,
    require_detached_payload: bool,
    trust_policy: &dyn TrustPolicy,
) -> Result<VerificationResult, SignDiagnostic> {
    let parts: Vec<&str> = compact_jws.split('.').collect();
    if parts.len() != 3 {
        return Err(diag("JWS must use compact serialization."));
    }
    let header = decode_header(parts[0])?;
    if header.alg != "ES256" {
        return Err(diag("Only ES256 JWS signatures are supported."));
    }
    if require_detached_payload && !parts[1].is_empty() {
        return Err(diag("Detached JWS payload must be omitted."));
    }
    let decision = trust_policy.evaluate(&header);
    let expected_payload = base64url_no_pad(digest.as_bytes());
    let payload_segment = if parts[1].is_empty() {
        expected_payload.as_str()
    } else {
        let payload = decode_base64url(parts[1])?;
        if payload != digest.as_bytes() {
            return Ok(result(
                digest,
                header,
                CryptographicStatus::Invalid,
                decision.status,
            ));
        }
        parts[1]
    };

    let Some(key) = decision.es256_key else {
        return Ok(result(
            digest,
            header,
            CryptographicStatus::NotChecked,
            decision.status,
        ));
    };
    let signing_input = format!("{}.{}", parts[0], payload_segment);
    let signature = decode_base64url(parts[2])?;
    if signature.len() != 64 {
        return Ok(result(
            digest,
            header,
            CryptographicStatus::Invalid,
            decision.status,
        ));
    }
    let signature =
        Signature::from_slice(&signature).map_err(|_| diag("Invalid ES256 signature encoding."))?;
    let status = if key.key.verify(signing_input.as_bytes(), &signature).is_ok() {
        CryptographicStatus::Valid
    } else {
        CryptographicStatus::Invalid
    };
    Ok(result(digest, header, status, decision.status))
}

fn decode_header(input: &str) -> Result<JwsHeader, SignDiagnostic> {
    let bytes = decode_base64url(input)?;
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| diag("Invalid JWS protected header JSON."))?;
    let alg = raw
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| diag("JWS protected header is missing alg."))?;
    let kid = optional_header_string(&raw, "kid")?;
    let typ = optional_header_string(&raw, "typ")?;
    Ok(JwsHeader {
        alg: alg.to_string(),
        kid,
        typ,
    })
}

fn optional_header_string(
    raw: &serde_json::Value,
    name: &str,
) -> Result<Option<String>, SignDiagnostic> {
    match raw.get(name) {
        Some(value) => value
            .as_str()
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| diag("JWS protected header has a non-string field.")),
        None => Ok(None),
    }
}

fn result(
    digest: &str,
    header: JwsHeader,
    cryptographic_status: CryptographicStatus,
    trust_status: TrustStatus,
) -> VerificationResult {
    VerificationResult {
        digest: digest.to_string(),
        header,
        cryptographic_status,
        trust_status,
    }
}

fn diag(message: &str) -> SignDiagnostic {
    SignDiagnostic {
        code: "NODX-E017".to_string(),
        severity: "warning".to_string(),
        message: message.to_string(),
    }
}

fn decode_base64url(input: &str) -> Result<Vec<u8>, SignDiagnostic> {
    if input.len() % 4 == 1 {
        return Err(diag("Invalid base64url length."));
    }
    let mut bits = 0u32;
    let mut bit_len = 0u8;
    let mut out = Vec::new();
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'=' => return Err(diag("Base64url padding is not allowed.")),
            _ => return Err(diag("Invalid base64url character.")),
        };
        bits = (bits << 6) | value as u32;
        bit_len += 6;
        if bit_len >= 8 {
            bit_len -= 8;
            out.push(((bits >> bit_len) & 0xff) as u8);
        }
    }
    if bit_len > 0 && (bits & ((1 << bit_len) - 1)) != 0 {
        return Err(diag("Invalid base64url trailing bits."));
    }
    Ok(out)
}

fn base64url_no_pad(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push(ALPHABET[(n & 63) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let n = (input[i] as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        }
        2 => {
            let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::ecdsa::signature::Signer;

    const POSITIVE: &[u8] = include_bytes!("../../../spec/tests/signature/positive.nodx");
    const TAMPERED: &[u8] = include_bytes!("../../../spec/tests/signature/tampered.nodx");

    #[test]
    fn digest_uses_canonical_ast_not_trivia() {
        let plain = b"---\nschema: nodx/1.0\ntitle: Signed Fixture\n---\n\n# Signed Fixture\n\nThis fixture is used by nodx-sign positive signature tests.\n";
        let extra_trivia = b"---\ntitle: Signed Fixture\nschema: nodx/1.0\n---\n\n# Signed Fixture\n\nThis fixture is used by nodx-sign positive signature tests.\n";
        assert_eq!(
            digest_text_nodx(plain).unwrap(),
            digest_text_nodx(extra_trivia).unwrap()
        );
    }

    #[test]
    fn verifies_detached_es256_jws() {
        let (policy, jws) = signed_fixture(POSITIVE, true);
        let result = verify_detached_jws(POSITIVE, &jws, &policy).unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Valid);
        assert_eq!(result.trust_status, TrustStatus::Trusted);
        assert_eq!(result.header.kid.as_deref(), Some("fixture-es256"));
    }

    #[test]
    fn tampered_document_fails_crypto_but_keeps_trust_separate() {
        let (policy, jws) = signed_fixture(POSITIVE, true);
        let result = verify_detached_jws(TAMPERED, &jws, &policy).unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Invalid);
        assert_eq!(result.trust_status, TrustStatus::Trusted);
    }

    #[test]
    fn untrusted_key_can_still_be_cryptographically_valid() {
        let (trusted_policy, jws) = signed_fixture(POSITIVE, true);
        let untrusted_policy =
            StaticTrustPolicy::untrusted(Some("fixture-es256"), trusted_policy.key);
        let result = verify_detached_jws(POSITIVE, &jws, &untrusted_policy).unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Valid);
        assert_eq!(result.trust_status, TrustStatus::Untrusted);
    }

    #[test]
    fn missing_key_reports_not_checked() {
        let (_policy, jws) = signed_fixture(POSITIVE, true);
        let result = verify_detached_jws(POSITIVE, &jws, &NoKeyPolicy).unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::NotChecked);
        assert_eq!(result.trust_status, TrustStatus::Untrusted);
    }

    #[test]
    fn attached_payload_must_match_digest() {
        let (policy, jws) = signed_fixture(POSITIVE, false);
        let result = verify_compact_jws_for_digest(
            &digest_text_nodx(POSITIVE).unwrap(),
            &jws,
            false,
            &policy,
        )
        .unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Valid);

        let result = verify_compact_jws_for_digest(
            &digest_text_nodx(TAMPERED).unwrap(),
            &jws,
            false,
            &policy,
        )
        .unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Invalid);
    }

    #[test]
    fn verifies_packaged_signature_entry() {
        let (policy, jws) = signed_fixture(POSITIVE, true);
        let package = package_fixture(POSITIVE, &jws);
        let result =
            verify_packaged_signature(&package, "signatures/document.jws", &policy).unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Valid);
        assert_eq!(result.trust_status, TrustStatus::Trusted);
    }

    #[test]
    fn packaged_signature_detects_document_tamper() {
        let (policy, jws) = signed_fixture(POSITIVE, true);
        let package = package_fixture(TAMPERED, &jws);
        let result =
            verify_packaged_signature(&package, "signatures/document.jws", &policy).unwrap();
        assert_eq!(result.cryptographic_status, CryptographicStatus::Invalid);
        assert_eq!(result.trust_status, TrustStatus::Trusted);
    }

    #[test]
    fn rejects_unsupported_alg() {
        let digest = digest_text_nodx(POSITIVE).unwrap();
        let header = base64url_no_pad(br#"{"alg":"EdDSA","kid":"fixture-es256"}"#);
        let jws = format!("{}..AA", header);
        let err = verify_compact_jws_for_digest(&digest, &jws, true, &NoKeyPolicy).unwrap_err();
        assert_eq!(err.code, "NODX-E017");
        assert!(err.message.contains("ES256"));
    }

    struct NoKeyPolicy;

    impl TrustPolicy for NoKeyPolicy {
        fn evaluate(&self, _header: &JwsHeader) -> TrustDecision {
            TrustDecision::untrusted_without_key()
        }
    }

    fn signed_fixture(input: &[u8], detached: bool) -> (StaticTrustPolicy, String) {
        let signing_key = signing_key();
        let verifying_key = *signing_key.verifying_key();
        let public_key = Es256PublicKey { key: verifying_key };
        let policy = StaticTrustPolicy::trusted(Some("fixture-es256"), public_key);
        let digest = digest_text_nodx(input).unwrap();
        let protected = base64url_no_pad(br#"{"alg":"ES256","kid":"fixture-es256","typ":"JWT"}"#);
        let payload = base64url_no_pad(digest.as_bytes());
        let signing_input = format!("{}.{}", protected, payload);
        let signature: Signature = signing_key.sign(signing_input.as_bytes());
        let signature = base64url_no_pad(&signature.to_bytes());
        let jws = if detached {
            format!("{}..{}", protected, signature)
        } else {
            format!("{}.{}.{}", protected, payload, signature)
        };
        (policy, jws)
    }

    fn signing_key() -> SigningKey {
        SigningKey::from_slice(&[
            0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a, 0x19, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12,
            0x11, 0x10, 0x0f, 0x0e, 0x0d, 0x0c, 0x0b, 0x0a, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04,
            0x03, 0x02, 0x01, 0x01,
        ])
        .unwrap()
    }

    fn package_fixture(doc: &[u8], jws: &str) -> Vec<u8> {
        let manifest = format!(
            "schema: nodx-package/1.0\nentry: doc.nodx\nentries:\n  - path: doc.nodx\n    size: {}\n    sha256: {}\n  - path: signatures/document.jws\n    size: {}\n    sha256: {}\n",
            doc.len(),
            nodx_package::sha256_base64url(doc),
            jws.len(),
            nodx_package::sha256_base64url(jws.as_bytes())
        );
        build_zip(vec![
            ("mimetype", b"application/nodx+zip".to_vec(), 0o100644),
            ("manifest.yaml", manifest.into_bytes(), 0o100644),
            ("doc.nodx", doc.to_vec(), 0o100644),
            ("signatures/document.jws", jws.as_bytes().to_vec(), 0o100644),
        ])
    }

    fn build_zip(entries: Vec<(&str, Vec<u8>, u32)>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, data, mode) in entries {
            let local_offset = out.len() as u32;
            let crc = crc32_bytes(&data);
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&data);
            central.push((name.to_string(), data.len() as u32, crc, local_offset, mode));
        }
        let cd_offset = out.len() as u32;
        for (name, len, crc, local_offset, mode) in &central {
            out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&((*mode) << 16).to_le_bytes());
            out.extend_from_slice(&local_offset.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
        }
        let cd_size = out.len() as u32 - cd_offset;
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(central.len() as u16).to_le_bytes());
        out.extend_from_slice(&(central.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn crc32_bytes(input: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &byte in input {
            crc ^= byte as u32;
            for _ in 0..8 {
                let mask = 0u32.wrapping_sub(crc & 1);
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }
}
