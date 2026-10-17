use super::model::LoginFlow;
use crate::services::token;
use sqlx::Transaction;
use sqlx::{Sqlite, SqlitePool};

pub async fn admin_by_email(
  pool: &SqlitePool,
  email: &str,
) -> Result<Option<(String, String, String, String)>, sqlx::Error> {
  sqlx::query_as("SELECT id,email,password_hash,role FROM admins WHERE email=? COLLATE NOCASE")
    .bind(email)
    .fetch_optional(pool)
    .await
}
pub async fn password_hash(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT password_hash FROM admins WHERE id=?")
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Resolve an OIDC `(issuer, subject)` pair to an existing account.
pub async fn oidc_identity(
  pool: &SqlitePool,
  issuer: &str,
  subject: &str,
) -> Result<Option<(String, String, String)>, sqlx::Error> {
  sqlx::query_as(
    "SELECT a.id, a.email, a.role FROM oidc_identities i JOIN admins a ON a.id = i.admin_id WHERE i.issuer = ? AND i.subject = ?",
  )
  .bind(issuer)
  .bind(subject)
  .fetch_optional(pool)
  .await
}

/// Link an account to a provider identity. Assumes the caller checked that the
/// pair is not already linked.
pub async fn link_identity(
  tx: &mut Transaction<'_, Sqlite>,
  admin_id: &str,
  issuer: &str,
  subject: &str,
  email: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO oidc_identities(id,admin_id,issuer,subject,email,created_at,last_login_at) VALUES(?,?,?,?,?,?,?)")
    .bind(token::public_id("oidc_"))
    .bind(admin_id)
    .bind(issuer)
    .bind(subject)
    .bind(email)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Record a fresh sign-in for an already-linked identity.
pub async fn touch_identity(
  tx: &mut Transaction<'_, Sqlite>,
  issuer: &str,
  subject: &str,
  email: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE oidc_identities SET last_login_at=?, email=? WHERE issuer=? AND subject=?")
    .bind(now)
    .bind(email)
    .bind(issuer)
    .bind(subject)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Apply the role mapped from the provider's groups claim.
pub async fn set_role(
  tx: &mut Transaction<'_, Sqlite>,
  admin_id: &str,
  role: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE admins SET role=?, updated_at=? WHERE id=?")
    .bind(role)
    .bind(now)
    .bind(admin_id)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

pub async fn prune_login_flows(
  tx: &mut Transaction<'_, Sqlite>,
  cutoff: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM oidc_login_flows WHERE created_at < ?")
    .bind(cutoff)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

pub async fn create_login_flow(
  tx: &mut Transaction<'_, Sqlite>,
  flow: &LoginFlow,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO oidc_login_flows(state,nonce,code_verifier,redirect_to,created_at) VALUES(?,?,?,?,?)")
    .bind(&flow.state)
    .bind(&flow.nonce)
    .bind(&flow.code_verifier)
    .bind(&flow.redirect_to)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Consume a login flow by its single-use `state`, returning it if it existed.
/// The row is deleted in the same transaction whether or not it is used again.
pub async fn consume_login_flow(
  pool: &SqlitePool,
  state: &str,
) -> Result<Option<(LoginFlow, String)>, sqlx::Error> {
  let mut tx = pool.begin().await?;
  let row: Option<(String, String, Option<String>, String)> = sqlx::query_as(
    "SELECT nonce,code_verifier,redirect_to,created_at FROM oidc_login_flows WHERE state = ?",
  )
  .bind(state)
  .fetch_optional(&mut *tx)
  .await?;
  let Some((nonce, code_verifier, redirect_to, created_at)) = row else {
    tx.rollback().await?;
    return Ok(None);
  };
  sqlx::query("DELETE FROM oidc_login_flows WHERE state = ?")
    .bind(state)
    .execute(&mut *tx)
    .await?;
  tx.commit().await?;
  Ok(Some((
    LoginFlow {
      state: state.to_owned(),
      nonce,
      code_verifier,
      redirect_to,
    },
    created_at,
  )))
}
