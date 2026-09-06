use super::model::*;
use crate::{
  extractors::{require_admin_browser, require_mutation, require_recent_browser_auth},
  http::HttpError,
  models::{AdminRole, AuthIdentity},
  modules::common,
  services::token,
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::Utc;

fn actor(identity: &AuthIdentity) -> (&str, &str) {
  match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => (admin_id, email),
    _ => ("", ""),
  }
}

fn human_role(role: Option<AdminRole>) -> Result<AdminRole, HttpError> {
  match role.unwrap_or(AdminRole::Admin) {
    AdminRole::Root => Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root role is reserved for the setup account.",
    )),
    AdminRole::AiAgent => Err(HttpError::bad_request(
      "ROLE_INVALID",
      "AI agents must be created as service accounts.",
    )),
    value => Ok(value),
  }
}

fn parse_role(value: &str) -> Result<AdminRole, HttpError> {
  match value {
    "root" => Ok(AdminRole::Root),
    "admin" => Ok(AdminRole::Admin),
    "member" => Ok(AdminRole::Member),
    _ => Err(HttpError::internal()),
  }
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<UserResponse>, HttpError> {
  require_admin_browser(identity)?;
  let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
    "SELECT id,email,role,created_at,updated_at FROM admins ORDER BY CASE role WHEN 'root' THEN 0 WHEN 'admin' THEN 1 ELSE 2 END, email",
  ).fetch_all(state.db.pool()).await?;
  rows
    .into_iter()
    .map(|(id, email, r, created_at, updated_at)| {
      Ok(UserResponse {
        id,
        email,
        role: parse_role(&r)?,
        created_at,
        updated_at,
      })
    })
    .collect()
}

pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<UserResponse, HttpError> {
  require_admin_browser(identity)?;
  sqlx::query_as::<_, (String, String, String, String, String)>(
    "SELECT id,email,role,created_at,updated_at FROM admins WHERE id=?",
  )
  .bind(id)
  .fetch_optional(state.db.pool())
  .await?
  .map(|(id, email, r, created_at, updated_at)| {
    Ok::<UserResponse, HttpError>(UserResponse {
      id,
      email,
      role: parse_role(&r)?,
      created_at,
      updated_at,
    })
  })
  .transpose()?
  .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: CreateUserRequest,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let email = common::validate_email(&request.email)?;
  common::validate_password(&request.password)?;
  let requested_role = human_role(request.role)?;
  let id = token::public_id("usr_");
  let now = Utc::now().to_rfc3339();
  let hash = common::hash_password_async(request.password).await?;
  let mut tx = state.db.pool().begin().await?;
  let inserted = sqlx::query(
    "INSERT INTO admins(id,email,password_hash,role,created_at,updated_at) VALUES(?,?,?,?,?,?)",
  )
  .bind(&id)
  .bind(&email)
  .bind(hash)
  .bind(requested_role.as_str())
  .bind(&now)
  .bind(&now)
  .execute(&mut *tx)
  .await;
  if let Err(error) = inserted {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "EMAIL_ALREADY_EXISTS",
        "An account with this email already exists.",
      ));
    }
    return Err(error.into());
  }
  let (actor_id, actor_email) = actor(identity);
  common::audit(
    &mut *tx,
    "admin",
    Some(actor_id),
    Some(actor_email),
    "admin.created",
    None,
    None,
    Some("admin"),
    Some(&id),
    serde_json::json!({"email":email,"role":requested_role.as_str()}),
  )
  .await?;
  tx.commit().await?;
  Ok(UserResponse {
    id,
    email,
    role: requested_role,
    created_at: now.clone(),
    updated_at: now,
  })
}

pub async fn update(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: UpdateUserRequest,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let (old_email, old_role, created_at): (String, String, String) =
    sqlx::query_as("SELECT email,role,created_at FROM admins WHERE id=?")
      .bind(id)
      .fetch_optional(state.db.pool())
      .await?
      .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  let (actor_id, _) = actor(identity);
  if old_role == "root" && id != actor_id {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  if id == actor_id && request.role.is_some() {
    return Err(HttpError::forbidden(
      "SELF_ROLE_CHANGE",
      "You cannot change your own role.",
    ));
  }
  let new_role = match request.role {
    Some(value) => human_role(Some(value))?,
    None => parse_role(&old_role)?,
  };
  if old_role == "root" && new_role != AdminRole::Root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root role cannot be changed.",
    ));
  }
  if old_role == "root" && request.password.is_some() {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "Use the root password change flow to change the root password.",
    ));
  }
  let email = match request.email {
    Some(value) => common::validate_email(&value)?,
    None => old_email,
  };
  let password_hash = if let Some(password) = request.password {
    common::validate_password(&password)?;
    Some(common::hash_password_async(password).await?)
  } else {
    None
  };
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let result = if let Some(hash) = password_hash {
    sqlx::query("UPDATE admins SET email=?,password_hash=?,role=?,updated_at=? WHERE id=?")
      .bind(&email)
      .bind(hash)
      .bind(new_role.as_str())
      .bind(&now)
      .bind(id)
      .execute(&mut *tx)
      .await
  } else {
    sqlx::query("UPDATE admins SET email=?,role=?,updated_at=? WHERE id=?")
      .bind(&email)
      .bind(new_role.as_str())
      .bind(&now)
      .bind(id)
      .execute(&mut *tx)
      .await
  };
  if let Err(error) = result {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "EMAIL_ALREADY_EXISTS",
        "An account with this email already exists.",
      ));
    }
    return Err(error.into());
  }
  sqlx::query("UPDATE sessions SET revoked_at=? WHERE admin_id=? AND revoked_at IS NULL")
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
  let (_, actor_email) = actor(identity);
  common::audit(
    &mut *tx,
    "admin",
    Some(actor_id),
    Some(actor_email),
    "admin.updated",
    None,
    None,
    Some("admin"),
    Some(id),
    serde_json::json!({"email":email,"role":new_role.as_str()}),
  )
  .await?;
  tx.commit().await?;
  Ok(UserResponse {
    id: id.into(),
    email,
    role: new_role,
    created_at,
    updated_at: now,
  })
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_admin_browser(identity)?;
  let (actor_id, actor_email) = actor(identity);
  if id == actor_id {
    return Err(HttpError::forbidden(
      "SELF_DELETE_FORBIDDEN",
      "You cannot delete your own account.",
    ));
  }
  let target_role: Option<String> = sqlx::query_scalar("SELECT role FROM admins WHERE id=?")
    .bind(id)
    .fetch_optional(state.db.pool())
    .await?;
  match target_role.as_deref() {
    None => {
      return Err(HttpError::not_found(
        "USER_NOT_FOUND",
        "The user was not found.",
      ));
    }
    Some("root") => {
      return Err(HttpError::forbidden(
        "ROOT_PROTECTED",
        "The root administrator cannot be deleted.",
      ));
    }
    Some("admin") | Some("member") => {}
    _ => return Err(HttpError::internal()),
  }
  let mut tx = state.db.pool().begin().await?;
  sqlx::query("DELETE FROM admins WHERE id=? AND role IN ('admin','member')")
    .bind(id)
    .execute(&mut *tx)
    .await?;
  common::audit(
    &mut *tx,
    "admin",
    Some(actor_id),
    Some(actor_email),
    "admin.deleted",
    None,
    None,
    Some("admin"),
    Some(id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
