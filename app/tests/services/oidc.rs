use std::{
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{SystemTime, UNIX_EPOCH},
};

use app::services::oidc::{OidcError, OidcVerifier, TrustPolicy, TrustedIssuer};
use axum::{Router, extract::State, routing::get};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use tokio::sync::RwLock;

const AUDIENCE: &str = "dopbase";
const KID: &str = "test-key-1";
const SUBJECT: &str = "workload-123";

// A fixed RSA key pair used only by these tests. The private key signs the
// assertions; the public modulus below is published through the mock JWKS.
const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCf3Mq2g6GfOYRZ
aMC57gnCfhqtbvsdCtAyGP5AgAqBWbn+BfIxtlIonT/FXwtrg63AUUZvCprFxD+r
ITUMaI45mPFIOM/MdqfDOUhJwzs1edczbyQoavyw/hIkxT9356+GABkdGf+llfqH
K68g0l9C23DjmW4cs2sRoi5ZPDPcy+Wpev2TJzUaeBaMgTX7zeI7lvfixjgGnTh+
MJ7Jb0ECgA4izrOnTuUQ+xJpVUIntDrQusHllG1OwAXu3jchjFhFpVnE1hCAhPhc
n3DO86VBnRy2CqCgGLMcut9p5JWx52gZjOj8DTWWklBQyPn71U+r/JGb5ryUXakf
tMzrGV+5AgMBAAECggEABPUzgjqN2iLOBHnXtEx6GAKSMDvHZOyvbUZSKEzwRney
aXrkun71iFXsIuJiN8jjLBo5LXtuNmDyOovymbpqYsXoAb3lT+mEJW1S3RrFbNCl
CG7Td79tq82NYX+OitjwHB9HqBvkzk1a8jM8IK/1jVzB4QUioZg841SczF6uGxRZ
M/tzcwEwBbv99eAhz3kmpF4LNfT8iI9WztbpNFL+uAZqdouxFSGWXS08niRjgAsJ
+o+faQCT7jSsKJHGoU7l7MvAdLcGDBe22yefMK9j0PRdNFoe+vHiLuUS0BHvumAg
zrkW+bjdQos1OHbB+Uu/ARSlYK3mXpC14v4LM118wQKBgQDM+S4Bh76Iy68opkfh
2igR9B8+0no4O+SnIg5Fo1R/DUFdp5y1GgNzpcSQqVYomvvBDxig27R/kMHf9hLd
wMTrsxXrpZHTugk7Lo3O1XZwnDAAWTAMd8tKdDraoQJ/2yVMpJpYqUbYDAt9i9lc
su+bK4HSXAGIjBxC9MiXNSVjCQKBgQDHqLlgPvgxg15ZKko+9UEJVWzRb34hOgRI
SNXzuhMRrxJrFfjr3sPa3xPFBjbxMLTyN9PiXHBHkvd9anm0zzyIAYC5koQwSpNM
d+FZYwcf0g3On1Du1eIUnMEMIofNYGFaIydgS73dpaxzNkvyK6X/d0QWruNe7HkD
dS5PmwnTMQKBgG1Y2TQzyG+Nyamz21EfYpUBWNHY0x7AxOBsiJSZ52/IEID389+h
Owos+sn5rF8JHxkFBqx5yLbs8KCb08C9H9Blv2Hw9/StoFMUKM6YjhDRIcJt0fY0
yhy0GNmdX4n5zNIwVa7Kz2JeOpsFSjXrt2teMGmab676n/EQSRpxXbqRAoGAfrOG
uBodvKYGGIVqylsu43JvdNTQ8Nr1Xr3B8EIz2bQE9ynhZiVemqWX0R8pfJORMGjX
MD3DDtin79HC3EfqvNBfVOQyuMaKhStKOURLgBj3pBib7oSSTMxRAZdkIKO6A85t
LYHDQkXhktdxCIREekLAVbi0F0fvX49cTS4w5PECgYBxr0ri5DDTUleMQbDvgthO
Q3F6VPigdK6HNMbyFIWhtK5Zyii4BshyRJmjA25X32j0pB+8Nhmz8nhcujTnQtjT
hKibh8B7wndFNTAO9bs/dMuJ3r7wqQ042ZqUL+WI0qI9rTkmeJ9W253W5eTRJGV2
UzLB0S9UP2/1VtzOuhy6rg==
-----END PRIVATE KEY-----";

const JWK_N: &str = "n9zKtoOhnzmEWWjAue4Jwn4arW77HQrQMhj-QIAKgVm5_gXyMbZSKJ0_xV8La4OtwFFGbwqaxcQ_qyE1DGiOOZjxSDjPzHanwzlIScM7NXnXM28kKGr8sP4SJMU_d-evhgAZHRn_pZX6hyuvINJfQttw45luHLNrEaIuWTwz3MvlqXr9kyc1GngWjIE1-83iO5b34sY4Bp04fjCeyW9BAoAOIs6zp07lEPsSaVVCJ7Q60LrB5ZRtTsAF7t43IYxYRaVZxNYQgIT4XJ9wzvOlQZ0ctgqgoBizHLrfaeSVsedoGYzo_A01lpJQUMj5-9VPq_yRm-a8lF2pH7TM6xlfuQ";

/// A mock identity provider that serves a discovery document and a JWKS, and
/// counts how often each endpoint is fetched. Its issuer identifier is its own
/// loopback URL so that discovery resolves back to it.
#[derive(Clone)]
struct Provider {
  issuer: String,
  jwks: Arc<RwLock<Value>>,
  jwks_requests: Arc<AtomicUsize>,
  discovery_requests: Arc<AtomicUsize>,
}

impl Provider {
  fn jwks_uri(&self) -> String {
    format!("{}/jwks", self.issuer)
  }

  async fn publish(
    &self,
    kid: &str,
  ) {
    *self.jwks.write().await = jwks_document(kid);
  }
}

async fn start_provider() -> Provider {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let provider = Provider {
    issuer: format!("http://{address}"),
    jwks: Arc::new(RwLock::new(jwks_document(KID))),
    jwks_requests: Arc::new(AtomicUsize::new(0)),
    discovery_requests: Arc::new(AtomicUsize::new(0)),
  };
  let router = Router::new()
    .route("/.well-known/openid-configuration", get(discovery))
    .route("/jwks", get(keys))
    .with_state(provider.clone());
  tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  provider
}

async fn keys(State(provider): State<Provider>) -> axum::Json<Value> {
  provider.jwks_requests.fetch_add(1, Ordering::SeqCst);
  axum::Json(provider.jwks.read().await.clone())
}

async fn discovery(State(provider): State<Provider>) -> axum::Json<Value> {
  provider.discovery_requests.fetch_add(1, Ordering::SeqCst);
  axum::Json(json!({
    "issuer": provider.issuer,
    "jwks_uri": provider.jwks_uri(),
  }))
}

fn jwks_document(kid: &str) -> Value {
  json!({
    "keys": [{
      "kty": "RSA",
      "use": "sig",
      "alg": "RS256",
      "kid": kid,
      "n": JWK_N,
      "e": "AQAB",
    }],
  })
}

fn policy(
  provider: &Provider,
  jwks_uri: Option<String>,
) -> TrustPolicy {
  TrustPolicy {
    issuers: vec![TrustedIssuer {
      issuer: provider.issuer.clone(),
      audiences: vec![AUDIENCE.to_owned()],
      algorithms: vec![Algorithm::RS256],
      jwks_uri,
    }],
  }
}

fn verifier(provider: &Provider) -> OidcVerifier {
  OidcVerifier::new(policy(provider, Some(provider.jwks_uri()))).unwrap()
}

fn now() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs()
}

fn claims(issuer: &str) -> Value {
  let now = now();
  json!({
    "iss": issuer,
    "sub": SUBJECT,
    "aud": AUDIENCE,
    "exp": now + 600,
    "iat": now,
    "nbf": now - 5,
    "groups": ["deploy", "ci"],
    "email": "ci@example.com",
  })
}

fn sign_with(
  header: &Header,
  claims: &Value,
) -> String {
  let key = EncodingKey::from_rsa_pem(PRIVATE_PEM.as_bytes()).unwrap();
  encode(header, claims, &key).unwrap()
}

fn sign(claims: &Value) -> String {
  let mut header = Header::new(Algorithm::RS256);
  header.kid = Some(KID.to_owned());
  sign_with(&header, claims)
}

#[tokio::test]
async fn accepts_a_token_signed_by_a_trusted_issuer() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let verified = verifier
    .verify(&sign(&claims(&provider.issuer)))
    .await
    .unwrap();

  assert_eq!(verified.issuer, provider.issuer);
  assert_eq!(verified.subject.as_deref(), Some(SUBJECT));
  assert_eq!(verified.audiences, vec![AUDIENCE.to_owned()]);
  assert_eq!(verified.groups("groups"), vec!["deploy", "ci"]);
  assert_eq!(verified.string("email"), Some("ci@example.com"));
  assert!(verified.expires_at.unwrap() > now() as i64);
}

#[tokio::test]
async fn reads_a_scalar_group_claim() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut claims = claims(&provider.issuer);
  claims["groups"] = json!("deploy");
  let verified = verifier.verify(&sign(&claims)).await.unwrap();

  assert_eq!(verified.groups("groups"), vec!["deploy"]);
}

#[tokio::test]
async fn accepts_a_token_without_a_key_id_when_one_key_is_published() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let token = sign_with(&Header::new(Algorithm::RS256), &claims(&provider.issuer));
  assert!(verifier.verify(&token).await.is_ok());
}

#[tokio::test]
async fn rejects_a_token_from_an_untrusted_issuer() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut claims = claims(&provider.issuer);
  claims["iss"] = json!("https://evil.example");
  assert_eq!(
    verifier.verify(&sign(&claims)).await.unwrap_err(),
    OidcError::UntrustedIssuer
  );
}

#[tokio::test]
async fn rejects_a_token_for_another_audience() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut claims = claims(&provider.issuer);
  claims["aud"] = json!("another-service");
  assert_eq!(
    verifier.verify(&sign(&claims)).await.unwrap_err(),
    OidcError::BadAudience
  );
}

#[tokio::test]
async fn rejects_a_token_without_an_audience() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut claims = claims(&provider.issuer);
  claims.as_object_mut().unwrap().remove("aud");
  assert_eq!(
    verifier.verify(&sign(&claims)).await.unwrap_err(),
    OidcError::BadAudience
  );
}

#[tokio::test]
async fn rejects_an_expired_token() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut claims = claims(&provider.issuer);
  claims["exp"] = json!(now() - 3600);
  assert_eq!(
    verifier.verify(&sign(&claims)).await.unwrap_err(),
    OidcError::Expired
  );
}

#[tokio::test]
async fn rejects_a_token_that_is_not_valid_yet() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut claims = claims(&provider.issuer);
  claims["nbf"] = json!(now() + 3600);
  assert_eq!(
    verifier.verify(&sign(&claims)).await.unwrap_err(),
    OidcError::NotYetValid
  );
}

#[tokio::test]
async fn rejects_a_token_with_a_broken_signature() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut token = sign(&claims(&provider.issuer));
  // Flip the first character of the signature. It carries real key material,
  // unlike the final base64 character, whose low bits are discarded.
  let index = token.rfind('.').unwrap() + 1;
  let replacement = if token.as_bytes()[index] == b'A' {
    'B'
  } else {
    'A'
  };
  token.replace_range(index..index + 1, &replacement.to_string());
  assert_eq!(
    verifier.verify(&token).await.unwrap_err(),
    OidcError::BadSignature
  );
}

#[tokio::test]
async fn rejects_a_token_that_uses_a_symmetric_algorithm() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let token = encode(
    &Header::new(Algorithm::HS256),
    &claims(&provider.issuer),
    &EncodingKey::from_secret(b"shared-secret"),
  )
  .unwrap();
  assert_eq!(
    verifier.verify(&token).await.unwrap_err(),
    OidcError::UnsupportedAlgorithm
  );
}

#[tokio::test]
async fn rejects_an_unknown_key_after_refreshing() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  let mut header = Header::new(Algorithm::RS256);
  header.kid = Some("rotated-unknown".to_owned());
  let token = sign_with(&header, &claims(&provider.issuer));
  assert_eq!(
    verifier.verify(&token).await.unwrap_err(),
    OidcError::UnknownKey
  );
  // One fetch for the cache miss and one after the key was not found.
  assert_eq!(provider.jwks_requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn caches_the_signing_keys_between_tokens() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  verifier
    .verify(&sign(&claims(&provider.issuer)))
    .await
    .unwrap();
  verifier
    .verify(&sign(&claims(&provider.issuer)))
    .await
    .unwrap();
  assert_eq!(provider.jwks_requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn refreshes_the_signing_keys_when_the_provider_rotates() {
  let provider = start_provider().await;
  let verifier = verifier(&provider);

  verifier
    .verify(&sign(&claims(&provider.issuer)))
    .await
    .unwrap();

  provider.publish("rotated-key-1").await;
  let mut header = Header::new(Algorithm::RS256);
  header.kid = Some("rotated-key-1".to_owned());
  let token = sign_with(&header, &claims(&provider.issuer));
  verifier.verify(&token).await.unwrap();

  assert_eq!(provider.jwks_requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn discovers_the_jwks_uri_from_issuer_metadata() {
  let provider = start_provider().await;
  let verifier = OidcVerifier::new(policy(&provider, None)).unwrap();

  verifier
    .verify(&sign(&claims(&provider.issuer)))
    .await
    .unwrap();

  assert_eq!(provider.discovery_requests.load(Ordering::SeqCst), 1);
  assert_eq!(provider.jwks_requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn reports_a_provider_failure_when_the_keys_cannot_be_fetched() {
  let provider = start_provider().await;
  let verifier = OidcVerifier::new(TrustPolicy {
    issuers: vec![TrustedIssuer {
      issuer: provider.issuer.clone(),
      audiences: vec![AUDIENCE.to_owned()],
      algorithms: vec![Algorithm::RS256],
      jwks_uri: Some(format!("{}/missing", provider.issuer)),
    }],
  })
  .unwrap();

  assert_eq!(
    verifier
      .verify(&sign(&claims(&provider.issuer)))
      .await
      .unwrap_err(),
    OidcError::ProviderUnavailable
  );
}
