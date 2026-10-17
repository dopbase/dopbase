//! End-to-end tests for browser single sign-on over OpenID Connect.
//!
//! A mock provider serves discovery, a JWKS, and a token endpoint that returns a
//! signed ID token. The tests drive the real router, then assert on the session,
//! the stored identity link, and the audit trail.

use std::{
  collections::BTreeMap,
  sync::{Arc, atomic::Ordering},
  time::{SystemTime, UNIX_EPOCH},
};

use app::{
  config::{MasterKeyConfig, ServerConfig},
  models::AdminRole,
  server,
  services::oidc::{OidcConfig, OidcLogin, TrustedIssuer},
  state::AppState,
};
use axum::{
  Router,
  body::{Body, to_bytes},
  extract::State,
  http::{HeaderMap, Request, header},
  routing::{get, post},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::RwLock;
use tower::ServiceExt;

const AUDIENCE: &str = "dopbase";
const KID: &str = "sso-key-1";

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

/// A mock provider: discovery, JWKS, and a token endpoint returning a signed ID
/// token whose claims the test sets through `claims`.
#[derive(Clone)]
struct Provider {
  issuer: String,
  claims: Arc<RwLock<Value>>,
  token_status: Arc<RwLock<u16>>,
  token_requests: Arc<std::sync::atomic::AtomicUsize>,
}

impl Provider {
  fn authorize_url(&self) -> String {
    format!("{}/authorize", self.issuer)
  }

  fn token_url(&self) -> String {
    format!("{}/token", self.issuer)
  }

  fn jwks_url(&self) -> String {
    format!("{}/jwks", self.issuer)
  }

  async fn set_claims(
    &self,
    claims: Value,
  ) {
    *self.claims.write().await = claims;
  }

  /// Build the ID token the next token request returns, injecting the
  /// registered claims and the nonce the flow sent.
  async fn issue(
    &self,
    nonce: &str,
  ) -> String {
    let now = now();
    let mut claims = self.claims.read().await.clone();
    let object = claims.as_object_mut().unwrap();
    object.insert("iss".into(), json!(self.issuer));
    object.insert("aud".into(), json!(AUDIENCE));
    object.insert("iat".into(), json!(now));
    object.insert("exp".into(), json!(now + 600));
    object.insert("nbf".into(), json!(now - 5));
    object.insert("nonce".into(), json!(nonce));
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KID.to_owned());
    let key = EncodingKey::from_rsa_pem(PRIVATE_PEM.as_bytes()).unwrap();
    encode(&header, &claims, &key).unwrap()
  }
}

async fn start_provider() -> Provider {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let provider = Provider {
    issuer: format!("http://{address}"),
    claims: Arc::new(RwLock::new(json!({}))),
    token_status: Arc::new(RwLock::new(200)),
    token_requests: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
  };
  let router = Router::new()
    .route("/.well-known/openid-configuration", get(discovery))
    .route("/jwks", get(keys))
    .route("/authorize", get(authorize))
    .route("/token", post(token))
    .with_state(provider.clone());
  tokio::spawn(async move {
    axum::serve(listener, router).await.unwrap();
  });
  provider
}

async fn discovery(State(provider): State<Provider>) -> axum::Json<Value> {
  axum::Json(json!({
    "issuer": provider.issuer,
    "authorization_endpoint": provider.authorize_url(),
    "token_endpoint": provider.token_url(),
    "jwks_uri": provider.jwks_url(),
  }))
}

async fn keys(State(_provider): State<Provider>) -> axum::Json<Value> {
  axum::Json(json!({
    "keys": [{
      "kty": "RSA",
      "use": "sig",
      "alg": "RS256",
      "kid": KID,
      "n": JWK_N,
      "e": "AQAB",
    }],
  }))
}

async fn authorize(State(provider): State<Provider>) -> axum::response::Redirect {
  axum::response::Redirect::temporary(&provider.token_url())
}

async fn token(
  State(provider): State<Provider>,
  body: String,
) -> axum::response::Response {
  use axum::response::IntoResponse;
  provider.token_requests.fetch_add(1, Ordering::SeqCst);
  let status = *provider.token_status.read().await;
  if status != 200 {
    return (axum::http::StatusCode::from_u16(status).unwrap(), ()).into_response();
  }
  // The exchange must present an authorization code with PKCE and the client id.
  assert!(body.contains("grant_type=authorization_code"), "{body}");
  assert!(body.contains("client_id=dopbase"), "{body}");
  assert!(body.contains("code_verifier="), "{body}");
  let nonce = body
    .split('&')
    .find_map(|pair| pair.strip_prefix("code="))
    .map(unescape)
    .expect("code parameter");
  let id_token = provider.issue(&nonce).await;
  axum::Json(json!({"id_token": id_token})).into_response()
}

fn unescape(value: &str) -> String {
  value
    .replace("%2F", "/")
    .replace("%2B", "+")
    .replace("%3D", "=")
}

fn now() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs()
}

fn login_config(provider: &Provider) -> OidcConfig {
  let mut roles = BTreeMap::new();
  roles.insert("dopbase-admins".to_owned(), AdminRole::Admin);
  roles.insert("dopbase-members".to_owned(), AdminRole::Member);
  OidcConfig {
    issuers: vec![TrustedIssuer {
      issuer: provider.issuer.clone(),
      audiences: vec![AUDIENCE.to_owned()],
      algorithms: vec![Algorithm::RS256],
      jwks_uri: Some(provider.jwks_url()),
    }],
    login: Some(OidcLogin {
      issuer: provider.issuer.clone(),
      client_id: "dopbase".to_owned(),
      client_secret: Some("s3cret".to_owned()),
      redirect_uri: None,
      scopes: vec!["openid".into(), "profile".into(), "email".into()],
      groups_claim: "groups".to_owned(),
      email_claim: "email".to_owned(),
      require_email_verified: true,
      token_endpoint_auth_method: None,
      roles,
    }),
  }
}

async fn app_with(oidc: OidcConfig) -> (TempDir, AppState, Router) {
  let directory = TempDir::new().unwrap();
  let database = directory.path().join("dopbase.db");
  let config = ServerConfig {
    data_dir: directory.path().to_path_buf(),
    database_url: format!("sqlite://{}", database.display()),
    master_key: MasterKeyConfig {
      provider: "file".into(),
      path: directory.path().join("master.key"),
    },
    public_url: "https://dopbase.example.com".to_owned(),
    oidc,
    ..ServerConfig::default()
  };
  let state = server::build_setup_state(config).await.unwrap();
  let router = server::router(state.clone());
  (directory, state, router)
}

async fn insert_account(
  state: &AppState,
  id: &str,
  email: &str,
  role: &str,
) {
  sqlx::query(
    "INSERT INTO admins(id,email,password_hash,role,created_at,updated_at) VALUES(?,?,?,?,?,?)",
  )
  .bind(id)
  .bind(email)
  .bind("unused-password-hash")
  .bind(role)
  .bind("2026-01-01T00:00:00+00:00")
  .bind("2026-01-01T00:00:00+00:00")
  .execute(state.db.pool())
  .await
  .unwrap();
}

async fn send(
  router: &Router,
  method: &str,
  path: &str,
  cookie: Option<&str>,
) -> (u16, HeaderMap) {
  let mut builder = Request::builder().method(method).uri(path);
  if let Some(cookie) = cookie {
    builder = builder.header(header::COOKIE, cookie);
  }
  let request = builder.body(Body::empty()).unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  let status = response.status().as_u16();
  let headers = response.headers().clone();
  let _ = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  (status, headers)
}

async fn session(
  router: &Router,
  cookie: &str,
) -> (u16, Value) {
  let request = Request::builder()
    .uri("/api/v1/auth/session")
    .header(header::COOKIE, cookie)
    .body(Body::empty())
    .unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  let status = response.status().as_u16();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  (
    status,
    serde_json::from_slice(&bytes).unwrap_or(Value::Null),
  )
}

fn location(headers: &HeaderMap) -> String {
  headers
    .get(header::LOCATION)
    .and_then(|value| value.to_str().ok())
    .unwrap_or_default()
    .to_owned()
}

fn session_cookie(headers: &HeaderMap) -> String {
  headers
    .get(header::SET_COOKIE)
    .and_then(|value| value.to_str().ok())
    .expect("session cookie")
    .split(';')
    .next()
    .unwrap()
    .to_owned()
}

fn query_param(
  url: &str,
  name: &str,
) -> String {
  let query = url.split_once('?').map(|(_, query)| query).unwrap_or("");
  for pair in query.split('&') {
    if let Some((key, value)) = pair.split_once('=')
      && key == name
    {
      return value.to_owned();
    }
  }
  panic!("missing {name} in {url}");
}

/// Start a sign-in and return the provider state and the flow nonce.
async fn begin(
  router: &Router,
  redirect: Option<&str>,
) -> (String, String) {
  let path = match redirect {
    Some(redirect) => format!("/api/v1/auth/oidc/start?redirect={redirect}"),
    None => "/api/v1/auth/oidc/start".to_owned(),
  };
  let (status, headers) = send(router, "GET", &path, None).await;
  assert_eq!(status, 303, "start redirects to the provider");
  let url = location(&headers);
  (query_param(&url, "state"), query_param(&url, "nonce"))
}

async fn audit_events(state: &AppState) -> Vec<(String, Value)> {
  sqlx::query_as::<_, (String, String)>(
    "SELECT action, metadata FROM audit_events WHERE action LIKE 'oidc.%' ORDER BY created_at, action",
  )
  .fetch_all(state.db.pool())
  .await
  .unwrap()
  .into_iter()
  .map(|(action, metadata)| (action, serde_json::from_str(&metadata).unwrap()))
  .collect()
}

#[tokio::test]
async fn config_reports_sso_availability() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;

  let request = Request::builder()
    .uri("/api/v1/auth/oidc/config")
    .body(Body::empty())
    .unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body: Value = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(body["data"]["enabled"], true);
  assert_eq!(body["data"]["issuer"], provider.issuer);
  state.db.close().await;
}

#[tokio::test]
async fn disabled_sso_hides_itself_and_refuses_to_start() {
  let (_directory, state, router) = app_with(OidcConfig::default()).await;

  let request = Request::builder()
    .uri("/api/v1/auth/oidc/config")
    .body(Body::empty())
    .unwrap();
  let response = router.clone().oneshot(request).await.unwrap();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body: Value = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(body["data"]["enabled"], false);

  let (status, _) = send(&router, "GET", "/api/v1/auth/oidc/start", None).await;
  assert_eq!(status, 404);
  state.db.close().await;
}

#[tokio::test]
async fn first_sign_in_links_the_account_and_applies_the_mapped_role() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_alice", "alice@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "alice-provider-id",
      "email": "alice@example.com",
      "email_verified": true,
      "groups": ["dopbase-admins"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, Some("%2Fprojects")).await;
  let url = format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}");
  let (status, headers) = send(&router, "GET", &url, None).await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/projects");
  let cookie = session_cookie(&headers);

  // The account is now an admin because the provider group maps to admin.
  let role: String = sqlx::query_scalar("SELECT role FROM admins WHERE id='usr_alice'")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(role, "admin");
  let linked: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM oidc_identities WHERE issuer=? AND subject='alice-provider-id'",
  )
  .bind(&provider.issuer)
  .fetch_one(state.db.pool())
  .await
  .unwrap();
  assert_eq!(linked, 1);

  let (status, body) = session(&router, &cookie).await;
  assert_eq!(status, 200);
  assert_eq!(body["data"]["email"], "alice@example.com");
  assert_eq!(body["data"]["role"], "admin");

  let events = audit_events(&state).await;
  assert!(
    events
      .iter()
      .any(|(action, _)| action == "oidc.identity.linked"),
    "{events:?}"
  );
  let succeeded = events
    .iter()
    .find(|(action, _)| action == "oidc.login.succeeded")
    .expect("succeeded event");
  assert_eq!(succeeded.1["role"], "admin");
  assert_eq!(succeeded.1["roleChanged"], true);
  state.db.close().await;
}

#[tokio::test]
async fn linked_identity_signs_in_again_without_a_new_email_match() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_bob", "bob@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "bob-provider-id",
      "email": "bob@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (first_state, nonce) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={first_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/");

  // The provider now reports a different email, but the stored subject link holds.
  provider
    .set_claims(json!({
      "sub": "bob-provider-id",
      "email": "bob.new@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;
  let (second_state, nonce) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={second_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/");

  let links: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM oidc_identities WHERE subject='bob-provider-id'")
      .fetch_one(state.db.pool())
      .await
      .unwrap();
  assert_eq!(links, 1);
  let events = audit_events(&state).await;
  assert_eq!(
    events
      .iter()
      .filter(|(action, _)| action == "oidc.login.succeeded")
      .count(),
    2
  );
  assert_eq!(
    events
      .iter()
      .filter(|(action, _)| action == "oidc.identity.linked")
      .count(),
    1
  );
  state.db.close().await;
}

#[tokio::test]
async fn unknown_email_is_denied_without_provisioning() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  provider
    .set_claims(json!({
      "sub": "stranger",
      "email": "nobody@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-denied");
  assert!(headers.get(header::SET_COOKIE).is_none());

  let accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(accounts, 0);
  let events = audit_events(&state).await;
  assert_eq!(events.len(), 1);
  assert_eq!(events[0].0, "oidc.login.failed");
  assert_eq!(events[0].1["reason"], "account_unknown");
  state.db.close().await;
}

#[tokio::test]
async fn unmapped_group_is_denied() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_carol", "carol@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "carol-provider-id",
      "email": "carol@example.com",
      "email_verified": true,
      "groups": ["some-other-group"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-denied");

  let events = audit_events(&state).await;
  assert_eq!(events[0].1["reason"], "role_unmapped");
  let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(sessions, 0);
  state.db.close().await;
}

#[tokio::test]
async fn unverified_email_is_denied() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_dave", "dave@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "dave-provider-id",
      "email": "dave@example.com",
      "email_verified": false,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-denied");

  let events = audit_events(&state).await;
  assert_eq!(events[0].1["reason"], "email_unverified");
  state.db.close().await;
}

#[tokio::test]
async fn unknown_state_returns_to_the_login_screen() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;

  let (status, headers) = send(
    &router,
    "GET",
    "/api/v1/auth/oidc/callback?code=x&state=unknown",
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-failed");

  let events = audit_events(&state).await;
  assert_eq!(events[0].0, "oidc.login.failed");
  assert_eq!(events[0].1["reason"], "state_unknown");
  assert_eq!(provider.token_requests.load(Ordering::SeqCst), 0);
  state.db.close().await;
}

#[tokio::test]
async fn a_replayed_state_cannot_sign_in_twice() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_erin", "erin@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "erin-provider-id",
      "email": "erin@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, None).await;
  let path = format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}");
  let (status, headers) = send(&router, "GET", &path, None).await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/");

  // The single-use state is gone: the same URL fails and mints no session.
  let (status, headers) = send(&router, "GET", &path, None).await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-failed");
  let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(sessions, 1);
  state.db.close().await;
}

#[tokio::test]
async fn an_external_redirect_is_ignored() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_frank", "frank@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "frank-provider-id",
      "email": "frank@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, Some("https%3A%2F%2Fevil.example")).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/");
  state.db.close().await;
}

#[tokio::test]
async fn root_account_keeps_its_role_across_sign_in() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_root", "root@example.com", "root").await;
  provider
    .set_claims(json!({
      "sub": "root-provider-id",
      "email": "root@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, None).await;
  let (status, _) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);

  let role: String = sqlx::query_scalar("SELECT role FROM admins WHERE id='usr_root'")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(role, "root");
  state.db.close().await;
}

#[tokio::test]
async fn a_provider_error_is_reported_as_a_failure() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;

  let (flow_state, _) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?error=access_denied&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-failed");

  let events = audit_events(&state).await;
  assert_eq!(events[0].1["reason"], "provider_error");
  state.db.close().await;
}

#[tokio::test]
async fn a_provider_failure_does_not_leak_a_token() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  *provider.token_status.write().await = 400;

  let (flow_state, nonce) = begin(&router, None).await;
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-failed");

  let events = audit_events(&state).await;
  assert_eq!(events[0].1["reason"], "oidc_exchange_failed");
  state.db.close().await;
}

#[tokio::test]
async fn a_wrong_nonce_is_rejected() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_gina", "gina@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "gina-provider-id",
      "email": "gina@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, _nonce) = begin(&router, None).await;
  // Present a code that is not the flow's nonce.
  let (status, headers) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code=not-the-nonce&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);
  assert_eq!(location(&headers), "/login?notice=oidc-failed");

  let events = audit_events(&state).await;
  assert_eq!(events[0].1["reason"], "nonce_mismatch");
  state.db.close().await;
}

#[tokio::test]
async fn sign_in_never_persists_the_provider_tokens() {
  let provider = start_provider().await;
  let (_directory, state, router) = app_with(login_config(&provider)).await;
  insert_account(&state, "usr_hana", "hana@example.com", "member").await;
  provider
    .set_claims(json!({
      "sub": "hana-provider-id",
      "email": "hana@example.com",
      "email_verified": true,
      "groups": ["dopbase-members"],
    }))
    .await;

  let (flow_state, nonce) = begin(&router, None).await;
  let (status, _) = send(
    &router,
    "GET",
    &format!("/api/v1/auth/oidc/callback?code={nonce}&state={flow_state}"),
    None,
  )
  .await;
  assert_eq!(status, 303);

  // No table stores the ID token, the code, or the PKCE verifier after the flow.
  let flows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oidc_login_flows")
    .fetch_one(state.db.pool())
    .await
    .unwrap();
  assert_eq!(flows, 0);
  let token_stored: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM oidc_identities WHERE subject LIKE '%eyJ%' OR email LIKE '%eyJ%'",
  )
  .fetch_one(state.db.pool())
  .await
  .unwrap();
  assert_eq!(token_stored, 0);
  state.db.close().await;
}
