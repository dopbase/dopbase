use crate::{constants::errors::TOKEN_SCOPE_INVALID, http::HttpError, models::AuthIdentity};
use sqlx::SqlitePool;

pub async fn runner_may_access_environment(
  pool: &SqlitePool,
  identity: &AuthIdentity,
  environment_id: &str,
) -> Result<(), HttpError> {
  let AuthIdentity::Runner {
    environment_id: scoped_env,
    workspace_id: scoped_workspace,
    ..
  } = identity
  else {
    return Ok(());
  };

  if let Some(scoped_env) = scoped_env {
    if scoped_env == environment_id {
      return Ok(());
    }
    return Err(HttpError::forbidden(
      TOKEN_SCOPE_INVALID,
      "The runner token cannot access this environment.",
    ));
  }

  if let Some(workspace_id) = scoped_workspace {
    let allowed: bool = sqlx::query_scalar(
      "SELECT EXISTS(
         SELECT 1 FROM environments e
         INNER JOIN project_locations l ON l.project_id = e.project_id
         WHERE e.id = ? AND l.workspace_id = ?)",
    )
    .bind(environment_id)
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .map_err(HttpError::from)?;
    if allowed {
      return Ok(());
    }
    return Err(HttpError::forbidden(
      TOKEN_SCOPE_INVALID,
      "The runner token cannot access this environment.",
    ));
  }

  Err(HttpError::forbidden(
    TOKEN_SCOPE_INVALID,
    "The runner token cannot access this environment.",
  ))
}
