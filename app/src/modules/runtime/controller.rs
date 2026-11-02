use super::{
  error::RuntimeError,
  model::{RuntimeSecretsQuery, RuntimeSnapshot},
  service::{self, RuntimeSecrets},
};
use crate::{
  http::{HttpError, HttpResponse, HttpResponseFormat},
  models::AuthIdentity,
  state::AppState,
};
use axum::{
  extract::{Query, State},
  http::{HeaderMap, HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
};

/// Fetch runtime secrets
///
/// Return every secret of the runner token's environment. The token decides
/// the environment; `reference` only checks it. Send the last
/// `revision` as `If-None-Match` to get `304 Not Modified` while nothing
/// has changed; a 304 decrypts nothing and writes no audit event.
#[utoipa::path(
  get,
  path = crate::constants::api::runtime::SECRETS,
  tag = "runtime",
  security(("bearerAuth" = [])),
  params(RuntimeSecretsQuery, ("If-None-Match" = Option<String>, Header, description = "The quoted revision from an earlier response's ETag")),
  responses(
    (status = 200, description = "Runtime secrets fetched", body = inline(HttpResponseFormat<RuntimeSnapshot>), headers(("ETag" = String, description = "The quoted revision"))),
    (status = 304, description = "Nothing changed since the revision in If-None-Match", headers(("ETag" = String, description = "The quoted revision"))),
    (status = 401, description = "The runner token is missing, revoked or expired", body = crate::http::ErrorBody),
    (status = 403, description = "The credential is not a runner token, or reference names another environment", body = crate::http::ErrorBody),
  ),
)]
pub async fn secrets(
  State(state): State<AppState>,
  identity: AuthIdentity,
  Query(query): Query<RuntimeSecretsQuery>,
  headers: HeaderMap,
) -> Result<Response, RuntimeError> {
  let if_none_match = headers
    .get(header::IF_NONE_MATCH)
    .and_then(|value| value.to_str().ok());
  Ok(
    match service::secrets(&state, &identity, query.reference.as_deref(), if_none_match).await? {
      RuntimeSecrets::Unchanged { revision } => {
        (StatusCode::NOT_MODIFIED, [(header::ETAG, etag(&revision)?)]).into_response()
      }
      RuntimeSecrets::Fetched(snapshot) => {
        let tag = etag(&snapshot.revision)?;
        HttpResponse::ok(snapshot, "RUNTIME_SECRETS_FETCHED")
          .with_header(header::ETAG, tag)
          .into_response()
      }
    },
  )
}

fn etag(revision: &str) -> Result<HeaderValue, HttpError> {
  HeaderValue::from_str(&format!("\"{revision}\"")).map_err(|_| HttpError::internal())
}
