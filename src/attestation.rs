//! Enclave attestation: prove the host on the other end of the connection is a
//! genuine FusedFrames enclave before any content is sent to it.
//!
//! Every request the CLI makes carries the caller's own content or returns it
//! (a guide body, a recording transcript, a search phrase), so the check runs in
//! [`crate::client::request`] before the request goes out, and one positive
//! result is cached per host for ten minutes.
//!
//! The check is mandatory for every non-loopback host and there is no override
//! flag in a shipped binary. Loopback hosts (`localhost`, `127.0.0.0/8`, `::1`)
//! skip it: nothing leaves the machine.
//!
//! What is verified, in order:
//! 1. TLS is opened to the host and the leaf certificate it presents is kept.
//! 2. `GET /.well-known/enclave-attestation` returns the COSE_Sign1 attestation
//!    document the Nitro Security Module minted.
//! 3. The document's certificate chain is verified up to the AWS Nitro
//!    Attestation root pinned in this binary.
//! 4. The COSE_Sign1 signature (ES384) is verified with the document's own
//!    signing certificate.
//! 5. PCR8, the signing-key measurement, matches one of [`ACCEPTED_PCR8`].
//! 6. The document's `user_data` equals SHA-256 of the DER of the leaf
//!    certificate captured in step 1, which is what binds the attestation to
//!    this connection rather than to a replayed one.
//! 7. The document's timestamp is within ten minutes of now.
//! 8. PCR0 is not all zeroes, which is what a debug-mode enclave reports.
//!
//! Anything that does not hold is a refusal. The reason stays inside this
//! module: the caller gets one sentence, so a probe cannot use the error text to
//! learn how far it got.
//!
//! The attestation is fetched on its own TLS connection, so a host that rotated
//! its certificate between that fetch and the request could still be refused on
//! the next check. That is the safe direction to be wrong in, and a rotation is
//! rare enough that a retry covers it.
//!
//! COSE_Sign1 is verified directly with RustCrypto primitives (`ciborium`,
//! `p384`, `sha2`, `x509-parser`) rather than through
//! `attestation-doc-validation` or the AWS Nitro SDK: both reach a C library
//! (OpenSSL) that this binary deliberately does not carry, since it is pure Rust
//! on rustls.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use ciborium::Value as Cbor;
use p384::ecdsa::signature::Verifier as _;
use p384::ecdsa::{DerSignature, Signature, VerifyingKey};
use sha2::{Digest as _, Sha256};
use url::Url;
use x509_parser::prelude::*;

use crate::client::is_loopback;
use crate::error::CliError;

/// The error kind every attestation refusal carries.
pub const UNVERIFIED_CODE: &str = "enclave_unverified";

/// The one sentence a person or an agent sees. The desktop app shows the same
/// words, so keep them identical in both.
pub const UNVERIFIED_MESSAGE: &str = "Could not verify the secure environment. Try again.";

/// How long one host stays verified before it is checked again.
const CACHE_TTL: Duration = Duration::from_secs(10 * 60);

/// How far the document's timestamp may sit from this machine's clock.
const MAX_CLOCK_SKEW: Duration = Duration::from_secs(10 * 60);

const ATTESTATION_PATH: [&str; 2] = [".well-known", "enclave-attestation"];

/// A document runs to a few kilobytes. Anything far larger is not one.
const MAX_DOCUMENT_BYTES: usize = 64 * 1024;

const ATTESTATION_TIMEOUT: Duration = Duration::from_secs(15);

/// COSE algorithm identifier for ES384 (ECDSA P-384 with SHA-384), the only
/// algorithm the Nitro Security Module signs with.
const COSE_ALG_ES384: i128 = -35;

/// Raw ES384 signatures are r || s over P-384.
const ES384_SIGNATURE_BYTES: usize = 96;

/// PCRs are SHA-384 digests.
const PCR_BYTES: usize = 48;

/// Longest chain the Nitro root is expected to sign through, as a cheap bound
/// on work a hostile host could ask us to do.
const MAX_CHAIN_CERTS: usize = 8;

/// OID 1.2.840.10045.4.3.3, ecdsa-with-SHA384: the only certificate signature
/// algorithm in a Nitro chain.
const OID_ECDSA_WITH_SHA384: &str = "1.2.840.10045.4.3.3";

/// The AWS Nitro Enclaves Attestation root, G1. Published at
/// <https://aws-nitro-enclaves.amazonaws.com/AWS_NitroEnclaves_Root-G1.zip>.
///
/// TODO: re-check this against the published SHA-256 of that zip,
/// `8cf60e2b2efca96c6a9e71e851d00c1b6991cc09eadbe64a6a1d1b1eb9faff7c`, at every
/// release, and again when AWS rotates to a G2 root. This certificate came from
/// that zip and its DER is SHA-256
/// `641a0321a3e244efe456463195d606317ed7cdcc3c1756e09893f3c68f79bb5b`, which
/// [`nitro_root_der_is_the_published_certificate`] asserts on every test run.
const NITRO_ROOT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIICETCCAZagAwIBAgIRAPkxdWgbkK/hHUbMtOTn+FYwCgYIKoZIzj0EAwMwSTEL
MAkGA1UEBhMCVVMxDzANBgNVBAoMBkFtYXpvbjEMMAoGA1UECwwDQVdTMRswGQYD
VQQDDBJhd3Mubml0cm8tZW5jbGF2ZXMwHhcNMTkxMDI4MTMyODA1WhcNNDkxMDI4
MTQyODA1WjBJMQswCQYDVQQGEwJVUzEPMA0GA1UECgwGQW1hem9uMQwwCgYDVQQL
DANBV1MxGzAZBgNVBAMMEmF3cy5uaXRyby1lbmNsYXZlczB2MBAGByqGSM49AgEG
BSuBBAAiA2IABPwCVOumCMHzaHDimtqQvkY4MpJzbolL//Zy2YlES1BR5TSksfbb
48C8WBoyt7F2Bw7eEtaaP+ohG2bnUs990d0JX28TcPQXCEPZ3BABIeTPYwEoCWZE
h8l5YoQwTcU/9KNCMEAwDwYDVR0TAQH/BAUwAwEB/zAdBgNVHQ4EFgQUkCW1DdkF
R+eWw5b6cp3PmanfS5YwDgYDVR0PAQH/BAQDAgGGMAoGCCqGSM49BAMDA2kAMGYC
MQCjfy+Rocm9Xue4YnwWmNJVA44fA0P5W2OpYow9OYCVRaEevL8uO1XYru5xtMPW
rfMCMQCi85sWBbJwKKXdS6BptQFuZbT73o/gBh1qUxl/nNr12UO8Yfwr6wPLb+6N
IwLz3/Y=
-----END CERTIFICATE-----";

/// The enclave signing-key measurements this build will talk to, PCR8 as lower
/// case hex.
///
/// TODO: replace the placeholder with the real PCR8 of the first signed enclave
/// build, and add the new value alongside the old one for the overlap whenever
/// the signing key changes. The placeholder is not hex, so it can never equal a
/// measurement and every non-loopback host is refused until then. That is the
/// correct behaviour: no enclave exists yet, so there is nothing safe to send
/// content to.
pub const ACCEPTED_PCR8: &[&str] = &["placeholder-until-the-first-signed-enclave-build"];

/// Hosts verified inside the last [`CACHE_TTL`], keyed by scheme, host and port.
///
/// The CLI builds a fresh reqwest client per request, so this is the only state
/// that survives between them. It lives for the length of one process: a new
/// `fusedframes` invocation verifies again.
static VERIFIED_HOSTS: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The pinned root in DER, decoded once.
static NITRO_ROOT_DER: LazyLock<Option<Vec<u8>>> = LazyLock::new(|| pem_to_der(NITRO_ROOT_PEM));

/// Refuse to send content unless `base` is a verified enclave.
///
/// Returns immediately for loopback hosts and for a host verified inside the
/// last ten minutes.
pub fn ensure_verified(base: &Url) -> Result<(), CliError> {
    if is_loopback(base) {
        return Ok(());
    }
    let key = host_key(base);
    if is_cached(&key) {
        return Ok(());
    }
    match verify_host(base) {
        Ok(()) => {
            remember(key);
            Ok(())
        }
        // The reason is deliberately dropped: the caller learns that the host is
        // not a verified enclave, not which step told us so.
        Err(_reason) => Err(CliError::new(UNVERIFIED_CODE, UNVERIFIED_MESSAGE)),
    }
}

/// Scheme, host and port together: two hosts that differ in any of them are
/// different endpoints and are verified separately.
fn host_key(url: &Url) -> String {
    let port = url
        .port_or_known_default()
        .map_or_else(|| "-".to_string(), |port| port.to_string());
    format!(
        "{}://{}:{port}",
        url.scheme(),
        url.host_str().unwrap_or_default()
    )
}

fn cache() -> std::sync::MutexGuard<'static, HashMap<String, Instant>> {
    // A poisoned lock means another thread panicked while holding it. The map is
    // a plain cache of instants, so its contents are still sound to read.
    VERIFIED_HOSTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn is_cached(key: &str) -> bool {
    let mut cache = cache();
    match cache.get(key) {
        Some(verified_at) if verified_at.elapsed() < CACHE_TTL => true,
        Some(_) => {
            cache.remove(key);
            false
        }
        None => false,
    }
}

fn remember(key: String) {
    cache().insert(key, Instant::now());
}

/// Open TLS, fetch the attestation document over that same connection and
/// verify it against the certificate the host presented.
fn verify_host(base: &Url) -> Result<(), String> {
    let mut url = base.clone();
    url.set_fragment(None);
    url.set_query(None);
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|()| "the API URL cannot carry a path".to_string())?;
        path.clear();
        path.extend(ATTESTATION_PATH);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(ATTESTATION_TIMEOUT)
        // The attestation must come from the host we are about to send content
        // to, so a redirect elsewhere is a refusal rather than something to
        // follow.
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("fusedframes-cli/", env!("CARGO_PKG_VERSION")))
        .no_proxy()
        // Hands back the leaf certificate the server presented on this
        // connection, which the document has to be bound to.
        .tls_info(true)
        .build()
        .map_err(|err| format!("could not set up the HTTP client: {err}"))?;

    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|err| format!("could not reach the attestation endpoint: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "the attestation endpoint answered HTTP {}",
            response.status().as_u16()
        ));
    }

    let leaf_der = response
        .extensions()
        .get::<reqwest::tls::TlsInfo>()
        .and_then(reqwest::tls::TlsInfo::peer_certificate)
        .ok_or_else(|| "the connection presented no certificate".to_string())?
        .to_vec();

    let body = response
        .text()
        .map_err(|err| format!("could not read the attestation reply: {err}"))?;
    let payload: AttestationResponse = serde_json::from_str(&body)
        .map_err(|err| format!("the attestation reply was not the expected JSON: {err}"))?;

    let document = base64::engine::general_purpose::STANDARD
        .decode(payload.attestation_document.trim())
        .map_err(|err| format!("the attestation document was not valid base64: {err}"))?;
    if document.len() > MAX_DOCUMENT_BYTES {
        return Err("the attestation document is implausibly large".to_string());
    }

    // The advertised fingerprint is a convenience for operators, so a mismatch
    // is a misconfiguration worth refusing, but `user_data` inside the signed
    // document is what actually binds the attestation to this connection.
    let leaf_sha256 = hex::encode(Sha256::digest(&leaf_der));
    if !payload
        .tls_certificate_sha256
        .trim()
        .eq_ignore_ascii_case(&leaf_sha256)
    {
        return Err("the advertised certificate fingerprint is not the one presented".to_string());
    }

    verify_document(&document, &leaf_der, SystemTime::now())
}

/// The attestation endpoint's JSON body.
#[derive(serde::Deserialize)]
struct AttestationResponse {
    attestation_document: String,
    tls_certificate_sha256: String,
}

/// Verify a COSE_Sign1 attestation document and everything it has to say about
/// the connection it arrived on.
fn verify_document(document: &[u8], leaf_der: &[u8], now: SystemTime) -> Result<(), String> {
    let sign1 = parse_cose_sign1(document)?;
    require_es384(&sign1.protected)?;

    let doc = parse_attestation_doc(&sign1.payload)?;
    let signing_key = verify_chain(&doc.cabundle, &doc.certificate)?;

    let signature = Signature::from_slice(&sign1.signature)
        .map_err(|err| format!("the document signature is malformed: {err}"))?;
    signing_key
        .verify(
            &sig_structure(&sign1.protected, &sign1.payload)?,
            &signature,
        )
        .map_err(|err| format!("the document signature does not verify: {err}"))?;

    // PCR0 measures the enclave image. A debug-mode enclave reports it as all
    // zeroes and its memory is readable from the parent instance, so it is never
    // somewhere content may go.
    let pcr0 = doc
        .pcrs
        .get(&0)
        .ok_or_else(|| "the document carries no PCR0".to_string())?;
    if pcr0.len() != PCR_BYTES {
        return Err("PCR0 is not a SHA-384 measurement".to_string());
    }
    if pcr0.iter().all(|byte| *byte == 0) {
        return Err("the enclave is running in debug mode".to_string());
    }

    // PCR8 measures the certificate the enclave image was signed with, which is
    // what ties a running enclave to a FusedFrames release.
    let pcr8 = doc
        .pcrs
        .get(&8)
        .ok_or_else(|| "the document carries no PCR8".to_string())?;
    if pcr8.len() != PCR_BYTES {
        return Err("PCR8 is not a SHA-384 measurement".to_string());
    }
    let pcr8_hex = hex::encode(pcr8);
    if !ACCEPTED_PCR8
        .iter()
        .any(|accepted| accepted.eq_ignore_ascii_case(&pcr8_hex))
    {
        return Err("PCR8 is not an accepted enclave build".to_string());
    }

    // Binds the document to this TLS connection: without it a document minted
    // for a genuine enclave could be replayed by anything sitting in front of
    // one.
    let user_data = doc
        .user_data
        .ok_or_else(|| "the document carries no user_data".to_string())?;
    if user_data.as_slice() != &Sha256::digest(leaf_der)[..] {
        return Err("the document is not bound to the presented certificate".to_string());
    }

    let minted_at = UNIX_EPOCH + Duration::from_millis(doc.timestamp);
    let drift = now
        .duration_since(minted_at)
        .or_else(|_| minted_at.duration_since(now))
        .map_err(|_| "the document timestamp cannot be read".to_string())?;
    if drift > MAX_CLOCK_SKEW {
        return Err("the document is too old, or this machine's clock is wrong".to_string());
    }

    Ok(())
}

/// The four fields of a COSE_Sign1 structure.
struct CoseSign1 {
    protected: Vec<u8>,
    payload: Vec<u8>,
    signature: Vec<u8>,
}

fn parse_cose_sign1(document: &[u8]) -> Result<CoseSign1, String> {
    let value: Cbor = ciborium::from_reader(document)
        .map_err(|err| format!("the attestation document is not CBOR: {err}"))?;
    // COSE_Sign1 may arrive tagged (18) or bare.
    let value = match value {
        Cbor::Tag(18, inner) => *inner,
        other => other,
    };
    let Cbor::Array(items) = value else {
        return Err("the attestation document is not a COSE_Sign1 array".to_string());
    };
    if items.len() != 4 {
        return Err("the COSE_Sign1 structure has the wrong number of fields".to_string());
    }
    let protected = as_bytes(&items[0], "the COSE protected header")?;
    let payload = as_bytes(&items[2], "the COSE payload")?;
    let signature = as_bytes(&items[3], "the COSE signature")?;
    if signature.len() != ES384_SIGNATURE_BYTES {
        return Err("the COSE signature is not ES384".to_string());
    }
    Ok(CoseSign1 {
        protected,
        payload,
        signature,
    })
}

/// The protected header must announce ES384 and nothing else: accepting the
/// algorithm the document names would let it pick a weaker one.
fn require_es384(protected: &[u8]) -> Result<(), String> {
    let header: Cbor = ciborium::from_reader(protected)
        .map_err(|err| format!("the COSE protected header is not CBOR: {err}"))?;
    let Cbor::Map(entries) = header else {
        return Err("the COSE protected header is not a map".to_string());
    };
    // Label 1 is `alg`.
    let alg = entries
        .iter()
        .find(|(key, _)| integer(key) == Some(1))
        .map(|(_, value)| value)
        .ok_or_else(|| "the COSE protected header names no algorithm".to_string())?;
    if integer(alg) != Some(COSE_ALG_ES384) {
        return Err("the COSE signature algorithm is not ES384".to_string());
    }
    Ok(())
}

/// The bytes a COSE_Sign1 signature is actually over, per RFC 8152: the
/// `Sig_structure` array, with an empty external AAD.
fn sig_structure(protected: &[u8], payload: &[u8]) -> Result<Vec<u8>, String> {
    let structure = Cbor::Array(vec![
        Cbor::Text("Signature1".to_string()),
        Cbor::Bytes(protected.to_vec()),
        Cbor::Bytes(Vec::new()),
        Cbor::Bytes(payload.to_vec()),
    ]);
    let mut encoded = Vec::new();
    ciborium::into_writer(&structure, &mut encoded)
        .map_err(|err| format!("could not rebuild the signed bytes: {err}"))?;
    Ok(encoded)
}

/// The fields of the attestation document this check reads.
struct AttestationDoc {
    timestamp: u64,
    pcrs: HashMap<u64, Vec<u8>>,
    certificate: Vec<u8>,
    cabundle: Vec<Vec<u8>>,
    user_data: Option<Vec<u8>>,
}

fn parse_attestation_doc(payload: &[u8]) -> Result<AttestationDoc, String> {
    let value: Cbor = ciborium::from_reader(payload)
        .map_err(|err| format!("the attestation payload is not CBOR: {err}"))?;
    let Cbor::Map(entries) = value else {
        return Err("the attestation payload is not a map".to_string());
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|(key, _)| matches!(key, Cbor::Text(text) if text == name))
            .map(|(_, value)| value)
    };

    let timestamp = field("timestamp")
        .and_then(integer)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| "the attestation payload carries no timestamp".to_string())?;

    let digest = field("digest").and_then(|value| match value {
        Cbor::Text(text) => Some(text.as_str()),
        _ => None,
    });
    if digest != Some("SHA384") {
        return Err("the attestation payload does not measure with SHA-384".to_string());
    }

    let Some(Cbor::Map(pcr_entries)) = field("pcrs") else {
        return Err("the attestation payload carries no PCRs".to_string());
    };
    let mut pcrs = HashMap::new();
    for (index, value) in pcr_entries {
        if let (Some(index), Cbor::Bytes(bytes)) = (
            integer(index).and_then(|index| u64::try_from(index).ok()),
            value,
        ) {
            pcrs.insert(index, bytes.clone());
        }
    }

    let certificate = field("certificate")
        .ok_or_else(|| "the attestation payload carries no signing certificate".to_string())
        .and_then(|value| as_bytes(value, "the signing certificate"))?;

    let Some(Cbor::Array(bundle)) = field("cabundle") else {
        return Err("the attestation payload carries no certificate bundle".to_string());
    };
    let cabundle = bundle
        .iter()
        .map(|value| as_bytes(value, "a bundle certificate"))
        .collect::<Result<Vec<_>, _>>()?;

    let user_data = match field("user_data") {
        Some(Cbor::Bytes(bytes)) => Some(bytes.clone()),
        _ => None,
    };

    Ok(AttestationDoc {
        timestamp,
        pcrs,
        certificate,
        cabundle,
        user_data,
    })
}

/// Verify the document's certificate chain and hand back the key the document
/// itself is signed with.
///
/// `cabundle` runs root first; the signing certificate is the leaf below it.
fn verify_chain(cabundle: &[Vec<u8>], certificate: &[u8]) -> Result<VerifyingKey, String> {
    let root_der = NITRO_ROOT_DER
        .as_ref()
        .ok_or_else(|| "the pinned Nitro root is not readable".to_string())?;
    let Some(claimed_root) = cabundle.first() else {
        return Err("the certificate bundle is empty".to_string());
    };
    // Byte equality with the pinned root, so nothing about the chain rests on a
    // name or a key the document itself chose.
    if claimed_root != root_der {
        return Err("the certificate bundle does not start at the pinned Nitro root".to_string());
    }
    if cabundle.len() >= MAX_CHAIN_CERTS {
        return Err("the certificate chain is too long".to_string());
    }

    let now = ASN1Time::now();
    let (_, root) = X509Certificate::from_der(root_der)
        .map_err(|err| format!("the pinned Nitro root does not parse: {err}"))?;
    if !root.validity().is_valid_at(now) {
        return Err("the pinned Nitro root is outside its validity window".to_string());
    }

    let mut issuer = root;
    let mut issuer_key = public_key(&issuer)?;
    let descendants = cabundle
        .iter()
        .skip(1)
        .map(Vec::as_slice)
        .chain(std::iter::once(certificate));

    for der in descendants {
        let (_, cert) = X509Certificate::from_der(der)
            .map_err(|err| format!("a chain certificate does not parse: {err}"))?;
        if cert.issuer().as_raw() != issuer.subject().as_raw() {
            return Err("a chain certificate names the wrong issuer".to_string());
        }
        if !cert.validity().is_valid_at(now) {
            return Err("a chain certificate is outside its validity window".to_string());
        }
        if cert.signature_algorithm.algorithm.to_id_string() != OID_ECDSA_WITH_SHA384 {
            return Err(
                "a chain certificate is not signed with ECDSA P-384 and SHA-384".to_string(),
            );
        }
        let signature = DerSignature::try_from(cert.signature_value.data.as_ref())
            .map_err(|err| format!("a chain signature is malformed: {err}"))?;
        issuer_key
            .verify(cert.tbs_certificate.as_ref(), &signature)
            .map_err(|err| format!("a chain signature does not verify: {err}"))?;

        issuer_key = public_key(&cert)?;
        issuer = cert;
    }

    Ok(issuer_key)
}

fn public_key(cert: &X509Certificate<'_>) -> Result<VerifyingKey, String> {
    VerifyingKey::from_sec1_bytes(cert.public_key().subject_public_key.data.as_ref())
        .map_err(|err| format!("a chain certificate has no usable P-384 key: {err}"))
}

fn as_bytes(value: &Cbor, what: &str) -> Result<Vec<u8>, String> {
    match value {
        Cbor::Bytes(bytes) => Ok(bytes.clone()),
        _ => Err(format!("{what} is not a byte string")),
    }
}

fn integer(value: &Cbor) -> Option<i128> {
    match value {
        Cbor::Integer(integer) => Some((*integer).into()),
        _ => None,
    }
}

/// Decode a single PEM certificate. Written by hand rather than pulled in as a
/// dependency because the only PEM this binary reads is the constant above.
fn pem_to_der(pem: &str) -> Option<Vec<u8>> {
    let body: String = pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("-----") && !line.is_empty())
        .collect();
    base64::engine::general_purpose::STANDARD.decode(body).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(url: &str) -> Url {
        Url::parse(url).expect("test URL parses")
    }

    /// A COSE protected header naming `alg`.
    fn protected_header(alg: i128) -> Vec<u8> {
        let mut encoded = Vec::new();
        ciborium::into_writer(
            &Cbor::Map(vec![(
                Cbor::Integer(1_i64.into()),
                Cbor::Integer(
                    ciborium::value::Integer::try_from(alg).expect("test algorithm fits"),
                ),
            )]),
            &mut encoded,
        )
        .expect("test CBOR encodes");
        encoded
    }

    #[test]
    fn nitro_root_der_is_the_published_certificate() {
        let der = NITRO_ROOT_DER.as_ref().expect("the pinned root decodes");
        assert_eq!(
            hex::encode(Sha256::digest(der)),
            "641a0321a3e244efe456463195d606317ed7cdcc3c1756e09893f3c68f79bb5b"
        );
        let (_, cert) = X509Certificate::from_der(der).expect("the pinned root parses");
        assert_eq!(cert.subject().as_raw(), cert.issuer().as_raw());
        assert!(cert.validity().is_valid_at(ASN1Time::now()));
        assert!(public_key(&cert).is_ok());
    }

    #[test]
    fn loopback_hosts_skip_verification() {
        assert!(ensure_verified(&parse("http://127.0.0.1:8081")).is_ok());
        assert!(ensure_verified(&parse("http://localhost:8081")).is_ok());
        assert!(ensure_verified(&parse("http://[::1]:8081")).is_ok());
        assert!(is_loopback(&parse("http://127.0.0.2")));
        assert!(!is_loopback(&parse("https://api.fusedframes.com")));
        assert!(!is_loopback(&parse("http://localhost.evil.com")));
    }

    #[test]
    fn host_keys_separate_scheme_host_and_port() {
        assert_eq!(
            host_key(&parse("https://api.fusedframes.com/guides")),
            "https://api.fusedframes.com:443"
        );
        assert_ne!(
            host_key(&parse("https://api.fusedframes.com")),
            host_key(&parse("https://api.fusedframes.com:8443"))
        );
        assert_ne!(
            host_key(&parse("https://api.fusedframes.com")),
            host_key(&parse("https://other.fusedframes.com"))
        );
    }

    #[test]
    fn the_cache_only_answers_for_hosts_it_has_seen() {
        let key = "https://cache-test.invalid:443".to_string();
        assert!(!is_cached(&key));
        remember(key.clone());
        assert!(is_cached(&key));
        cache().remove(&key);
    }

    #[test]
    fn a_document_that_is_not_cbor_is_refused() {
        assert!(verify_document(b"not cbor at all", b"leaf", SystemTime::now()).is_err());
    }

    #[test]
    fn a_cose_structure_of_the_wrong_shape_is_refused() {
        let mut encoded = Vec::new();
        ciborium::into_writer(
            &Cbor::Array(vec![Cbor::Bytes(Vec::new()), Cbor::Bytes(Vec::new())]),
            &mut encoded,
        )
        .expect("test CBOR encodes");
        assert!(parse_cose_sign1(&encoded).is_err());
    }

    #[test]
    fn only_es384_is_accepted() {
        let es384 = protected_header(COSE_ALG_ES384);
        assert!(require_es384(&es384).is_ok());
        // ES256, and an unsigned document, are both refused.
        assert!(require_es384(&protected_header(-7)).is_err());
        assert!(require_es384(&protected_header(0)).is_err());
        assert!(require_es384(b"").is_err());
    }

    #[test]
    fn the_signed_bytes_follow_rfc_8152() {
        let bytes = sig_structure(&[1, 2], &[3, 4]).expect("the structure encodes");
        let decoded: Cbor = ciborium::from_reader(bytes.as_slice()).expect("it decodes again");
        let Cbor::Array(items) = decoded else {
            panic!("Sig_structure is an array");
        };
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], Cbor::Text("Signature1".to_string()));
        assert_eq!(items[1], Cbor::Bytes(vec![1, 2]));
        assert_eq!(items[2], Cbor::Bytes(Vec::new()));
        assert_eq!(items[3], Cbor::Bytes(vec![3, 4]));
    }

    #[test]
    fn a_chain_that_does_not_start_at_the_pinned_root_is_refused() {
        assert!(verify_chain(&[], b"leaf").is_err());
        assert!(verify_chain(&[vec![0x30, 0x00]], b"leaf").is_err());
    }

    #[test]
    fn pcr8_is_a_placeholder_so_every_host_fails_closed() {
        // Until the first signed enclave build there is nothing to talk to, and
        // a value that is not hex can never match a measurement.
        assert_eq!(ACCEPTED_PCR8.len(), 1);
        assert!(hex::decode(ACCEPTED_PCR8[0]).is_err());
    }
}
