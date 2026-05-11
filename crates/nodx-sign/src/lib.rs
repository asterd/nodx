#![forbid(unsafe_code)]

use nodx_core::{
    Document, ResourceLimits, base64url_decode, base64url_encode, canonical_json, parse_bytes,
    sha256_base64url,
};
use nodx_package::Package;
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
    pub(crate) key: VerifyingKey,
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
            .map_err(|_| crypto_error("Invalid ES256 public key bytes."))?;
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
    sha256_base64url(canonical_json(doc).as_bytes())
}

pub fn digest_text_nodx(input: &[u8]) -> Result<String, SignDiagnostic> {
    digest_text_nodx_with_limits(input, ResourceLimits::default())
}

pub fn digest_text_nodx_with_limits(
    input: &[u8],
    limits: ResourceLimits,
) -> Result<String, SignDiagnostic> {
    let doc = parse_bytes_with_limits(input, limits).map_err(|err| diag_for(&err.message))?;
    Ok(canonical_digest(&doc))
}

fn parse_bytes_with_limits(
    input: &[u8],
    limits: ResourceLimits,
) -> Result<Document, nodx_core::Diagnostic> {
    nodx_core::parse_bytes_with_limits(input, limits)
}

pub fn verify_detached_jws(
    text_nodx: &[u8],
    compact_jws: &str,
    trust_policy: &dyn TrustPolicy,
) -> Result<VerificationResult, SignDiagnostic> {
    verify_detached_jws_with_limits(text_nodx, compact_jws, trust_policy, ResourceLimits::default())
}

pub fn verify_detached_jws_with_limits(
    text_nodx: &[u8],
    compact_jws: &str,
    trust_policy: &dyn TrustPolicy,
    limits: ResourceLimits,
) -> Result<VerificationResult, SignDiagnostic> {
    let digest = digest_text_nodx_with_limits(text_nodx, limits)?;
    verify_compact_jws_for_digest(&digest, compact_jws, true, trust_policy, limits)
}

pub fn verify_packaged_signature(
    package_bytes: &[u8],
    signature_path: &str,
    trust_policy: &dyn TrustPolicy,
) -> Result<VerificationResult, SignDiagnostic> {
    verify_packaged_signature_with_limits(
        package_bytes,
        signature_path,
        trust_policy,
        ResourceLimits::default(),
    )
}

pub fn verify_packaged_signature_with_limits(
    package_bytes: &[u8],
    signature_path: &str,
    trust_policy: &dyn TrustPolicy,
    limits: ResourceLimits,
) -> Result<VerificationResult, SignDiagnostic> {
    let package =
        Package::open(package_bytes, limits).map_err(|err| diag_for(&err.message))?;
    let digest = digest_text_nodx_with_limits(package.entry_bytes(), limits)?;
    let signature = package
        .fs()
        .read(signature_path)
        .ok_or_else(|| crypto_error("Package signature entry is missing."))?;
    let signature_text =
        std::str::from_utf8(signature).map_err(|_| crypto_error("Package signature is not UTF-8."))?;
    verify_compact_jws_for_digest(&digest, signature_text.trim(), false, trust_policy, limits)
}

pub fn verify_compact_jws_for_digest(
    digest: &str,
    compact_jws: &str,
    require_detached_payload: bool,
    trust_policy: &dyn TrustPolicy,
    limits: ResourceLimits,
) -> Result<VerificationResult, SignDiagnostic> {
    let parts: Vec<&str> = compact_jws.split('.').collect();
    if parts.len() != 3 {
        return Err(crypto_error("JWS must use compact serialization."));
    }
    let header = decode_header(parts[0], limits)?;
    if header.alg != "ES256" {
        return Err(crypto_error("Only ES256 JWS signatures are supported."));
    }
    if require_detached_payload && !parts[1].is_empty() {
        return Err(crypto_error("Detached JWS payload must be omitted."));
    }
    let decision = trust_policy.evaluate(&header);
    let expected_payload = base64url_encode(digest.as_bytes());
    let payload_segment = if parts[1].is_empty() {
        expected_payload.as_str()
    } else {
        let payload = decode_b64url(parts[1])?;
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
    let signature = decode_b64url(parts[2])?;
    if signature.len() != 64 {
        return Ok(result(
            digest,
            header,
            CryptographicStatus::Invalid,
            decision.status,
        ));
    }
    let signature = Signature::from_slice(&signature)
        .map_err(|_| crypto_error("Invalid ES256 signature encoding."))?;
    let status = if key.key.verify(signing_input.as_bytes(), &signature).is_ok() {
        CryptographicStatus::Valid
    } else {
        CryptographicStatus::Invalid
    };
    Ok(result(digest, header, status, decision.status))
}

fn decode_header(input: &str, limits: ResourceLimits) -> Result<JwsHeader, SignDiagnostic> {
    if input.len() > limits.signature_header_bytes {
        return Err(crypto_error("JWS protected header exceeds size limit."));
    }
    let bytes = decode_b64url(input)?;
    if bytes.len() > limits.signature_header_bytes {
        return Err(crypto_error("JWS protected header exceeds size limit."));
    }
    let raw: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| crypto_error("Invalid JWS protected header JSON."))?;
    let object = raw
        .as_object()
        .ok_or_else(|| crypto_error("JWS protected header must be a JSON object."))?;

    // Reject unsupported header members that would smuggle trust
    for forbidden in ["jku", "jwk", "x5u", "x5c", "x5t", "x5t#S256"] {
        if object.contains_key(forbidden) {
            return Err(crypto_error(&format!(
                "JWS protected header field `{forbidden}` is not allowed."
            )));
        }
    }
    if let Some(crit) = object.get("crit") {
        let crit_list = crit
            .as_array()
            .ok_or_else(|| crypto_error("JWS `crit` must be a JSON array."))?;
        if !crit_list.is_empty() {
            return Err(crypto_error(
                "JWS `crit` extensions are not supported by this verifier.",
            ));
        }
    }

    let alg = object
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| crypto_error("JWS protected header is missing alg."))?;
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
            .ok_or_else(|| crypto_error("JWS protected header has a non-string field.")),
        None => Ok(None),
    }
}

fn decode_b64url(input: &str) -> Result<Vec<u8>, SignDiagnostic> {
    base64url_decode(input).map_err(|_| crypto_error("Invalid base64url encoding."))
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

fn crypto_error(message: &str) -> SignDiagnostic {
    SignDiagnostic {
        // Errors that prevent verification are surfaced as NODX-E024-class issues
        // (required signature capability unsupported / malformed); the absence /
        // not-verified case uses NODX-E017 from verify_*.
        code: "NODX-E017".to_string(),
        severity: "error".to_string(),
        message: message.to_string(),
    }
}

fn diag_for(message: &str) -> SignDiagnostic {
    SignDiagnostic {
        code: "NODX-E017".to_string(),
        severity: "error".to_string(),
        message: message.to_string(),
    }
}

#[allow(dead_code)]
fn keep_parse_bytes_import_alive() {
    let _ = parse_bytes;
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
    fn rejects_unsupported_alg() {
        let digest = digest_text_nodx(POSITIVE).unwrap();
        let header = base64url_encode(br#"{"alg":"EdDSA","kid":"fixture-es256"}"#);
        let jws = format!("{}..AA", header);
        let err = verify_compact_jws_for_digest(
            &digest,
            &jws,
            true,
            &NoKeyPolicy,
            ResourceLimits::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("ES256"));
    }

    #[test]
    fn rejects_crit_extensions() {
        let digest = digest_text_nodx(POSITIVE).unwrap();
        let header = base64url_encode(
            br#"{"alg":"ES256","kid":"fixture-es256","crit":["evilext"],"evilext":"x"}"#,
        );
        let jws = format!("{}..AA", header);
        let err = verify_compact_jws_for_digest(
            &digest,
            &jws,
            true,
            &NoKeyPolicy,
            ResourceLimits::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("crit"));
    }

    #[test]
    fn rejects_jwk_and_x5u_smuggling() {
        let digest = digest_text_nodx(POSITIVE).unwrap();
        for bad in [
            r#"{"alg":"ES256","kid":"x","jwk":{}}"#,
            r#"{"alg":"ES256","kid":"x","x5u":"https://attacker.test/cert"}"#,
            r#"{"alg":"ES256","kid":"x","jku":"https://attacker.test/keys.json"}"#,
        ] {
            let header = base64url_encode(bad.as_bytes());
            let jws = format!("{}..AA", header);
            let err = verify_compact_jws_for_digest(
                &digest,
                &jws,
                true,
                &NoKeyPolicy,
                ResourceLimits::default(),
            )
            .unwrap_err();
            assert!(err.message.contains("not allowed"));
        }
    }

    #[test]
    fn rejects_oversize_header() {
        let big = vec![b'a'; 64 * 1024];
        let header = base64url_encode(&big);
        let jws = format!("{}..AA", header);
        let err = verify_compact_jws_for_digest(
            "sha256-AAAA",
            &jws,
            true,
            &NoKeyPolicy,
            ResourceLimits {
                signature_header_bytes: 1024,
                ..ResourceLimits::default()
            },
        )
        .unwrap_err();
        assert!(err.message.contains("header"));
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
        let protected = base64url_encode(br#"{"alg":"ES256","kid":"fixture-es256","typ":"JWT"}"#);
        let payload = base64url_encode(digest.as_bytes());
        let signing_input = format!("{}.{}", protected, payload);
        let signature: Signature = signing_key.sign(signing_input.as_bytes());
        let signature = base64url_encode(&signature.to_bytes());
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
}
