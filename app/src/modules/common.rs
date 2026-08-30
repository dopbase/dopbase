use std::{collections::BTreeMap, sync::OnceLock};

use argon2::{
  Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
  password_hash::{SaltString, rand_core::OsRng},
};
use regex::Regex;
use serde_json::Value;
use sqlx::{Executor, Sqlite};

use crate::{
  constants::errors::{EMAIL_INVAILD, EMAIL_INVAILD_MESSAGE},
  http::HttpError,
  services::token,
};

pub fn validate_slug(
  value: &str,
  code: &str,
  label: &str,
) -> Result<(), HttpError> {
  static SLUG: OnceLock<Regex> = OnceLock::new();
  let valid = SLUG.get_or_init(|| Regex::new(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$").unwrap());
  if value.len() > 63 || !valid.is_match(value) {
    return Err(HttpError::validation(BTreeMap::from([(
      code.into(),
      format!("{label} must be a lowercase slug of at most 63 characters."),
    )])));
  }
  Ok(())
}

pub fn validate_email(value: &str) -> Result<String, HttpError> {
  static EMAIL: OnceLock<Regex> = OnceLock::new();
  let normalized = value.trim().to_lowercase();
  let valid = EMAIL.get_or_init(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap());
  if normalized.len() > 254 || !valid.is_match(&normalized) {
    return Err(HttpError::validation(BTreeMap::from([(
      EMAIL_INVAILD.into(),
      EMAIL_INVAILD_MESSAGE.into(),
    )])));
  }
  Ok(normalized)
}

pub fn validate_password(value: &str) -> Result<(), HttpError> {
  let length = value.chars().count();
  let mut errors = BTreeMap::new();
  if length < 12 {
    errors.insert(
      "PASSWORD_TOO_SHORT".into(),
      "Password must contain at least 12 characters.".into(),
    );
  }
  if length > 128 {
    errors.insert(
      "PASSWORD_TOO_LONG".into(),
      "Password must contain at most 128 characters.".into(),
    );
  }
  if errors.is_empty() {
    Ok(())
  } else {
    Err(HttpError::validation(errors))
  }
}

pub fn hash_password(value: &str) -> Result<String, HttpError> {
  let params = Params::new(19_456, 2, 1, None).map_err(|_| HttpError::internal())?;
  let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
  let salt = SaltString::generate(&mut OsRng);
  argon
    .hash_password(value.as_bytes(), &salt)
    .map(|hash| hash.to_string())
    .map_err(|_| HttpError::internal())
}

pub fn verify_password(
  value: &str,
  encoded: &str,
) -> bool {
  PasswordHash::new(encoded).ok().is_some_and(|hash| {
    Argon2::default()
      .verify_password(value.as_bytes(), &hash)
      .is_ok()
  })
}

#[allow(clippy::too_many_arguments)]
pub async fn audit<'e, E>(
  executor: E,
  actor_type: &str,
  actor_id: Option<&str>,
  actor_label: Option<&str>,
  action: &str,
  project_id: Option<&str>,
  environment_id: Option<&str>,
  resource_type: Option<&str>,
  resource_id: Option<&str>,
  metadata: Value,
) -> Result<(), sqlx::Error>
where
  E: Executor<'e, Database = Sqlite>,
{
  sqlx::query("INSERT INTO audit_events(id, actor_type, actor_id, actor_label, action, project_id, environment_id, resource_type, resource_id, metadata, created_at) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(token::public_id("aud_")).bind(actor_type).bind(actor_id).bind(actor_label).bind(action)
        .bind(project_id).bind(environment_id).bind(resource_type).bind(resource_id).bind(metadata.to_string())
        .bind(chrono::Utc::now().to_rfc3339()).execute(executor).await.map(|_| ())
}
