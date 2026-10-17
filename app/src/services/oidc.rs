//! OIDC trust policy and assertion verification.
//!
//! This module is the shared foundation for OIDC authentication. Given a
//! configured set of trusted issuers, it verifies a signed JSON Web Token that
//! an identity provider issued and returns the verified claims. It starts no
//! login flow and mints no credentials: the browser SSO login and the CI
//! workload exchange are built on top of it.
//!
//! Verification fails closed. A token is accepted only when all of the
//! following hold:
//!
//! - its `iss` claim exactly matches a configured issuer;
//! - its signature verifies against that issuer's published JWKS;
//! - its header algorithm is one the issuer allows (asymmetric only);
//! - its `aud` claim contains one of the issuer's configured audiences; and
//! - its `exp`, and its `nbf` when present, are inside the clock-skew window.
//!
//! Rejections carry a stable [`OidcError`] code without echoing token material,
//! so callers can audit or report a failure without leaking the assertion.

use std::{
  collections::HashMap,
  sync::Arc,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use base64::{
  Engine,
  engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
pub use jsonwebtoken::Algorithm;
use jsonwebtoken::{
  DecodingKey, Validation, decode, decode_header,
  errors::{Error, ErrorKind},
  jwk::{Jwk, JwkSet},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::RwLock;
use url::{Host, Url};

/// Time a fetched JWKS document is reused before it is fetched again.
const JWKS_CACHE_TTL: Duration = Duration::from_secs(600);

/// Timeout applied to issuer metadata and JWKS requests.
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(10);

/// Clock skew tolerated when checking `exp` and `nbf`.
const CLOCK_SKEW_SECONDS: u64 = 60;

/// OpenID Connect discovery path appended to the issuer identifier.
const DISCOVERY_PATH: &str = ".well-known/openid-configuration";

/// Default signature algorithms when an issuer does not list any.
fn default_algorithms() -> Vec<Algorithm> {
  vec![Algorithm::RS256]
}

/// A trusted provider whose assertions Dopbase accepts.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TrustedIssuer {
  /// Exact issuer identifier the provider puts in the `iss` claim.
  pub issuer: String,
  /// Accepted audiences. A token must list at least one of these in `aud`.
  pub audiences: Vec<String>,
  /// Signature algorithms accepted for this issuer.
  #[serde(default = "default_algorithms")]
  pub algorithms: Vec<Algorithm>,
  /// JWKS endpoint. When omitted it is discovered from the issuer metadata.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub jwks_uri: Option<String>,
}

impl TrustedIssuer {
  fn validate(&self) -> Result<()> {
    validate_provider_url(&self.issuer).context("issuer")?;
    if self.audiences.is_empty() {
      bail!("at least one audience is required");
    }
    if self
      .audiences
      .iter()
      .any(|audience| audience.trim().is_empty())
    {
      bail!("audiences must not be blank");
    }
    if self.algorithms.is_empty() {
      bail!("at least one algorithm is required");
    }
    for algorithm in &self.algorithms {
      ensure_asymmetric(algorithm)?;
    }
    if let Some(jwks_uri) = &self.jwks_uri {
      validate_provider_url(jwks_uri).context("jwks_uri")?;
    }
    Ok(())
  }
}

/// The set of identity providers whose assertions Dopbase accepts.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TrustPolicy {
  #[serde(default)]
  pub issuers: Vec<TrustedIssuer>,
}

impl TrustPolicy {
  /// Reject a policy that can never verify a well-formed token.
  pub fn validate(&self) -> Result<()> {
    for (index, issuer) in self.issuers.iter().enumerate() {
      issuer
        .validate()
        .with_context(|| format!("oidc.issuers[{index}]"))?;
    }
    Ok(())
  }

  /// Find the trusted issuer that exactly matches an `iss` value.
  pub fn find(
    &self,
    issuer: &str,
  ) -> Option<&TrustedIssuer> {
    self
      .issuers
      .iter()
      .find(|candidate| candidate.issuer == issuer)
  }
}

/// Claims read from an assertion after its signature and registered claims were
/// verified. Values are available for callers that map claims to roles or to an
/// environment scope.
#[derive(Clone, Debug)]
pub struct VerifiedClaims {
  /// Verified `iss` value.
  pub issuer: String,
  /// `sub` value when present.
  pub subject: Option<String>,
  /// Verified `aud` values.
  pub audiences: Vec<String>,
  /// `exp` value when present.
  pub expires_at: Option<i64>,
  /// `iat` value when present.
  pub issued_at: Option<i64>,
  /// `nbf` value when present.
  pub not_before: Option<i64>,
  claims: Map<String, Value>,
}

impl VerifiedClaims {
  fn from_claims(
    issuer: &str,
    claims: Map<String, Value>,
  ) -> Self {
    Self {
      issuer: issuer.to_owned(),
      subject: claims.get("sub").and_then(Value::as_str).map(str::to_owned),
      audiences: string_values(&claims, "aud"),
      expires_at: claims.get("exp").and_then(Value::as_i64),
      issued_at: claims.get("iat").and_then(Value::as_i64),
      not_before: claims.get("nbf").and_then(Value::as_i64),
      claims,
    }
  }

  /// Raw value of a claim, for mappings the caller owns.
  pub fn claim(
    &self,
    name: &str,
  ) -> Option<&Value> {
    self.claims.get(name)
  }

  /// Read a claim that holds a single string.
  pub fn string(
    &self,
    name: &str,
  ) -> Option<&str> {
    self.claims.get(name).and_then(Value::as_str)
  }

  /// Read a claim that holds a string or an array of strings. Non-string
  /// members are ignored, so a malformed claim never fails a mapping.
  pub fn strings(
    &self,
    name: &str,
  ) -> Vec<&str> {
    match self.claims.get(name) {
      Some(Value::String(value)) => vec![value.as_str()],
      Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).collect(),
      _ => Vec::new(),
    }
  }

  /// Read a group or role membership claim as a list of values.
  pub fn groups(
    &self,
    claim: &str,
  ) -> Vec<&str> {
    self.strings(claim)
  }
}

/// Why an assertion was rejected. Codes are stable so callers can audit or
/// report a failure without leaking the assertion or provider internals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OidcError {
  /// The token is not a well-formed, correctly shaped assertion.
  Malformed,
  /// The `iss` value is not a configured provider.
  UntrustedIssuer,
  /// The signature algorithm is not allowed for this provider.
  UnsupportedAlgorithm,
  /// The signing key is not present in the provider's JWKS.
  UnknownKey,
  /// The signature did not verify against the provider's keys.
  BadSignature,
  /// The token has expired.
  Expired,
  /// The token is not valid yet.
  NotYetValid,
  /// The token audience is not accepted.
  BadAudience,
  /// The token issuer does not match the configured provider.
  BadIssuer,
  /// The provider's metadata or signing keys could not be fetched.
  ProviderUnavailable,
}

impl OidcError {
  /// Stable machine-readable code for API responses and audit records.
  pub fn code(self) -> &'static str {
    match self {
      OidcError::Malformed => "oidc_malformed_token",
      OidcError::UntrustedIssuer => "oidc_untrusted_issuer",
      OidcError::UnsupportedAlgorithm => "oidc_unsupported_algorithm",
      OidcError::UnknownKey => "oidc_unknown_key",
      OidcError::BadSignature => "oidc_invalid_signature",
      OidcError::Expired => "oidc_expired",
      OidcError::NotYetValid => "oidc_not_yet_valid",
      OidcError::BadAudience => "oidc_invalid_audience",
      OidcError::BadIssuer => "oidc_invalid_issuer",
      OidcError::ProviderUnavailable => "oidc_provider_unavailable",
    }
  }
}

impl std::fmt::Display for OidcError {
  fn fmt(
    &self,
    formatter: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    let message = match self {
      OidcError::Malformed => "the assertion is not a well-formed OIDC token",
      OidcError::UntrustedIssuer => "the assertion was issued by an untrusted provider",
      OidcError::UnsupportedAlgorithm => "the assertion uses an unsupported signature algorithm",
      OidcError::UnknownKey => "the assertion was signed with an unknown key",
      OidcError::BadSignature => "the assertion signature could not be verified",
      OidcError::Expired => "the assertion has expired",
      OidcError::NotYetValid => "the assertion is not valid yet",
      OidcError::BadAudience => "the assertion audience is not accepted",
      OidcError::BadIssuer => "the assertion issuer does not match the configured provider",
      OidcError::ProviderUnavailable => "the identity provider's signing keys are unavailable",
    };
    formatter.write_str(message)
  }
}

impl std::error::Error for OidcError {}

impl From<&Error> for OidcError {
  fn from(error: &Error) -> Self {
    match error.kind() {
      ErrorKind::ExpiredSignature => OidcError::Expired,
      ErrorKind::ImmatureSignature => OidcError::NotYetValid,
      ErrorKind::InvalidAudience => OidcError::BadAudience,
      ErrorKind::InvalidIssuer => OidcError::BadIssuer,
      ErrorKind::InvalidSignature => OidcError::BadSignature,
      ErrorKind::InvalidAlgorithm | ErrorKind::MissingAlgorithm => OidcError::UnsupportedAlgorithm,
      ErrorKind::MissingRequiredClaim(claim) => match claim.as_str() {
        "aud" => OidcError::BadAudience,
        "iss" => OidcError::BadIssuer,
        _ => OidcError::Malformed,
      },
      _ => OidcError::Malformed,
    }
  }
}

/// Verifies assertions against a [`TrustPolicy`], caching each provider's
/// signing keys. Cheap to clone; the cache is shared across clones.
#[derive(Clone)]
pub struct OidcVerifier {
  policy: Arc<TrustPolicy>,
  client: reqwest::Client,
  cache: Arc<RwLock<HashMap<String, CachedJwks>>>,
}

struct CachedJwks {
  keys: JwkSet,
  fetched_at: Instant,
}

impl OidcVerifier {
  /// Build a verifier with a dedicated HTTP client for provider requests.
  pub fn new(policy: TrustPolicy) -> Result<Self> {
    let client = reqwest::Client::builder()
      .timeout(PROVIDER_TIMEOUT)
      .build()
      .context("failed to build the OIDC HTTP client")?;
    Ok(Self::with_client(policy, client))
  }

  /// Build a verifier around an existing HTTP client, for callers that already
  /// configure one.
  pub fn with_client(
    policy: TrustPolicy,
    client: reqwest::Client,
  ) -> Self {
    Self {
      policy: Arc::new(policy),
      client,
      cache: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  /// The trust policy this verifier enforces.
  pub fn policy(&self) -> &TrustPolicy {
    &self.policy
  }

  /// Verify an assertion and return its claims. Returns [`OidcError`] when the
  /// token is not acceptable; it never falls back to an unverified result.
  pub async fn verify(
    &self,
    token: &str,
  ) -> Result<VerifiedClaims, OidcError> {
    let header = decode_header(token).map_err(|_| OidcError::Malformed)?;

    // The issuer is read from the unverified payload only to select which
    // provider's keys to use. The signature and the issuer are verified before
    // any claim is trusted, so a forged `iss` cannot widen the trust policy.
    let issuer = unverified_claim(token, "iss").ok_or(OidcError::Malformed)?;
    let trusted = self
      .policy
      .find(&issuer)
      .ok_or(OidcError::UntrustedIssuer)?;

    if !trusted.algorithms.contains(&header.alg) {
      return Err(OidcError::UnsupportedAlgorithm);
    }

    let mut keys = self.keys(trusted).await?;
    if select_keys(&keys, header.kid.as_deref()).is_empty() {
      // The provider may have rotated its signing keys: fetch once more.
      keys = self.refresh_keys(trusted).await?;
    }
    let candidates = select_keys(&keys, header.kid.as_deref());
    if candidates.is_empty() {
      return Err(OidcError::UnknownKey);
    }

    let validation = validation_for(trusted, header.alg);
    for jwk in candidates {
      let Ok(key) = DecodingKey::from_jwk(jwk) else {
        continue;
      };
      match decode::<Map<String, Value>>(token, &key, &validation) {
        Ok(data) => return Ok(VerifiedClaims::from_claims(&issuer, data.claims)),
        Err(error) => {
          if !is_key_mismatch(&error) {
            return Err(OidcError::from(&error));
          }
        }
      }
    }
    Err(OidcError::BadSignature)
  }

  async fn keys(
    &self,
    trusted: &TrustedIssuer,
  ) -> Result<JwkSet, OidcError> {
    if let Some(keys) = self.cached_keys(&trusted.issuer).await {
      return Ok(keys);
    }
    self.refresh_keys(trusted).await
  }

  async fn cached_keys(
    &self,
    issuer: &str,
  ) -> Option<JwkSet> {
    let cache = self.cache.read().await;
    let entry = cache.get(issuer)?;
    (entry.fetched_at.elapsed() < JWKS_CACHE_TTL).then(|| entry.keys.clone())
  }

  async fn refresh_keys(
    &self,
    trusted: &TrustedIssuer,
  ) -> Result<JwkSet, OidcError> {
    let uri = self.jwks_uri(trusted).await?;
    let response = self
      .client
      .get(uri)
      .send()
      .await
      .map_err(|_| OidcError::ProviderUnavailable)?;
    if !response.status().is_success() {
      return Err(OidcError::ProviderUnavailable);
    }
    let keys = response
      .json::<JwkSet>()
      .await
      .map_err(|_| OidcError::ProviderUnavailable)?;
    if keys.keys.is_empty() {
      return Err(OidcError::ProviderUnavailable);
    }
    let mut cache = self.cache.write().await;
    cache.insert(
      trusted.issuer.clone(),
      CachedJwks {
        keys: keys.clone(),
        fetched_at: Instant::now(),
      },
    );
    Ok(keys)
  }

  async fn jwks_uri(
    &self,
    trusted: &TrustedIssuer,
  ) -> Result<Url, OidcError> {
    let raw = match &trusted.jwks_uri {
      Some(uri) => uri.clone(),
      None => self.discover_jwks_uri(trusted).await?,
    };
    validate_provider_url(&raw).map_err(|_| OidcError::ProviderUnavailable)?;
    Url::parse(&raw).map_err(|_| OidcError::ProviderUnavailable)
  }

  async fn discover_jwks_uri(
    &self,
    trusted: &TrustedIssuer,
  ) -> Result<String, OidcError> {
    let base = trusted.issuer.trim_end_matches('/');
    let metadata_url = Url::parse(&format!("{base}/{DISCOVERY_PATH}"))
      .map_err(|_| OidcError::ProviderUnavailable)?;
    let response = self
      .client
      .get(metadata_url)
      .send()
      .await
      .map_err(|_| OidcError::ProviderUnavailable)?;
    if !response.status().is_success() {
      return Err(OidcError::ProviderUnavailable);
    }
    let metadata = response
      .json::<IssuerMetadata>()
      .await
      .map_err(|_| OidcError::ProviderUnavailable)?;
    if let Some(issuer) = metadata.issuer
      && issuer != trusted.issuer
    {
      return Err(OidcError::ProviderUnavailable);
    }
    metadata.jwks_uri.ok_or(OidcError::ProviderUnavailable)
  }
}

/// The subset of the OpenID Connect discovery document Dopbase needs.
#[derive(Debug, Deserialize)]
struct IssuerMetadata {
  #[serde(default)]
  issuer: Option<String>,
  #[serde(default)]
  jwks_uri: Option<String>,
}

/// Select the keys to try for a token: the key named by `kid`, or every key
/// when the token names none.
fn select_keys<'a>(
  set: &'a JwkSet,
  kid: Option<&str>,
) -> Vec<&'a Jwk> {
  match kid {
    Some(kid) => set
      .keys
      .iter()
      .filter(|key| key.common.key_id.as_deref() == Some(kid))
      .collect(),
    None => set.keys.iter().collect(),
  }
}

/// Build the claim validation for a token from the issuer's trust settings. The
/// algorithm is pinned to the one the token actually used, which the caller has
/// already checked against the issuer's allow-list.
fn validation_for(
  trusted: &TrustedIssuer,
  algorithm: Algorithm,
) -> Validation {
  let mut validation = Validation::new(algorithm);
  validation.leeway = CLOCK_SKEW_SECONDS;
  validation.validate_nbf = true;
  validation.set_audience(&trusted.audiences);
  validation.set_issuer(&[trusted.issuer.as_str()]);
  validation.set_required_spec_claims(&["exp", "iss", "aud"]);
  validation
}

/// True when a decode error means "this is not the right key", so verification
/// should continue with the next candidate key.
fn is_key_mismatch(error: &Error) -> bool {
  matches!(
    error.kind(),
    ErrorKind::InvalidSignature
      | ErrorKind::InvalidAlgorithm
      | ErrorKind::MissingAlgorithm
      | ErrorKind::InvalidKeyFormat
      | ErrorKind::InvalidRsaKey(_)
      | ErrorKind::InvalidEcdsaKey
  )
}

/// Read a claim from the unverified payload, for issuer selection only.
fn unverified_claim(
  token: &str,
  name: &str,
) -> Option<String> {
  let payload = token.split('.').nth(1)?;
  let decoded = URL_SAFE_NO_PAD
    .decode(payload)
    .or_else(|_| URL_SAFE.decode(payload))
    .ok()?;
  let claims = serde_json::from_slice::<Map<String, Value>>(&decoded).ok()?;
  claims.get(name).and_then(Value::as_str).map(str::to_owned)
}

fn string_values(
  claims: &Map<String, Value>,
  name: &str,
) -> Vec<String> {
  match claims.get(name) {
    Some(Value::String(value)) => vec![value.clone()],
    Some(Value::Array(values)) => values
      .iter()
      .filter_map(Value::as_str)
      .map(str::to_owned)
      .collect(),
    _ => Vec::new(),
  }
}

/// Reject a provider URL that is not HTTPS, except over a loopback host where a
/// local development provider is expected.
fn validate_provider_url(raw: &str) -> Result<()> {
  let url = Url::parse(raw).context("must be an absolute URL")?;
  if url.host_str().is_none() {
    bail!("must include a host");
  }
  match url.scheme() {
    "https" => Ok(()),
    "http" if url_is_loopback(&url) => Ok(()),
    "http" => bail!("must use HTTPS unless the host is loopback"),
    other => bail!("unsupported scheme \"{other}\": use HTTPS"),
  }
}

fn url_is_loopback(url: &Url) -> bool {
  match url.host() {
    Some(Host::Domain(host)) => host == "localhost",
    Some(Host::Ipv4(host)) => host.is_loopback(),
    Some(Host::Ipv6(host)) => host.is_loopback(),
    None => false,
  }
}

/// Reject symmetric algorithms: a shared secret cannot be verified from a
/// provider's published keys and would let an attacker sign with a public key.
fn ensure_asymmetric(algorithm: &Algorithm) -> Result<()> {
  match algorithm {
    Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => bail!(
      "symmetric algorithm {algorithm:?} is not supported: configure an asymmetric algorithm such as RS256"
    ),
    _ => Ok(()),
  }
}
