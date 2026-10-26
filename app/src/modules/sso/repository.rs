use crate::modules::common::NO_PASSWORD_HASH;
use sqlx::{SqliteConnection, SqlitePool};

#[derive(sqlx::FromRow)]
pub struct ProviderRow {
  pub id: String,
  pub preset: String,
  pub display_name: String,
  pub issuer: String,
  pub client_id: String,
  pub client_secret_ciphertext: Vec<u8>,
  pub client_secret_nonce: Vec<u8>,
  pub enabled: bool,
  pub require_sso: bool,
  pub hide_password_form: bool,
  pub verified_at: Option<String>,
  pub updated_at: String,
}

const PROVIDER_COLUMNS: &str = "id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,require_sso,hide_password_form,verified_at,updated_at";

/// Version 1 has one provider, so this is the first one created.
pub async fn provider(pool: &SqlitePool) -> Result<Option<ProviderRow>, sqlx::Error> {
  sqlx::query_as(&format!(
    "SELECT {PROVIDER_COLUMNS} FROM sso_providers ORDER BY created_at LIMIT 1"
  ))
  .fetch_optional(pool)
  .await
}

pub async fn provider_by_id(
  pool: &SqlitePool,
  id: &str,
) -> Result<Option<ProviderRow>, sqlx::Error> {
  sqlx::query_as(&format!(
    "SELECT {PROVIDER_COLUMNS} FROM sso_providers WHERE id=?"
  ))
  .bind(id)
  .fetch_optional(pool)
  .await
}

/// The enabled provider's button label, `require_sso`, and
/// `hide_password_form`.
pub async fn enabled_policy(
  pool: &SqlitePool
) -> Result<Option<(String, bool, bool)>, sqlx::Error> {
  sqlx::query_as(
    "SELECT display_name,require_sso,hide_password_form FROM sso_providers WHERE enabled=1 LIMIT 1",
  )
  .fetch_optional(pool)
  .await
}

/// The enabled provider's button label, when `admin_id` has an identity
/// linked to it.
pub async fn confirmation_name(
  pool: &SqlitePool,
  admin_id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar(
    "SELECT p.display_name FROM sso_providers p JOIN admin_identities i ON i.provider_id=p.id WHERE p.enabled=1 AND i.admin_id=? LIMIT 1",
  )
  .bind(admin_id)
  .fetch_optional(pool)
  .await
}

pub async fn linked_count(
  pool: &SqlitePool,
  provider_id: &str,
) -> Result<i64, sqlx::Error> {
  sqlx::query_scalar("SELECT COUNT(*) FROM admin_identities WHERE provider_id=?")
    .bind(provider_id)
    .fetch_one(pool)
    .await
}

/// Admins and members without a password, who can only sign in through
/// single sign-on.
pub async fn passwordless_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
  // TODO(rbac): counts the fixed admin and member roles. Count every role
  // that may sign in without a password once roles are configurable.
  sqlx::query_scalar(
    "SELECT COUNT(*) FROM admins WHERE password_hash=? AND role IN ('admin','member')",
  )
  .bind(NO_PASSWORD_HASH)
  .fetch_one(pool)
  .await
}

pub struct ProviderFields<'a> {
  pub id: &'a str,
  pub preset: &'a str,
  pub display_name: &'a str,
  pub issuer: &'a str,
  pub client_id: &'a str,
  pub client_secret_ciphertext: &'a [u8],
  pub client_secret_nonce: &'a [u8],
  pub now: &'a str,
}

/// A new provider starts disabled and unverified.
pub async fn insert_provider(
  conn: &mut SqliteConnection,
  fields: ProviderFields<'_>,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO sso_providers(id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,verified_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,0,NULL,?,?)")
    .bind(fields.id)
    .bind(fields.preset)
    .bind(fields.display_name)
    .bind(fields.issuer)
    .bind(fields.client_id)
    .bind(fields.client_secret_ciphertext)
    .bind(fields.client_secret_nonce)
    .bind(fields.now)
    .bind(fields.now)
    .execute(conn)
    .await?;
  Ok(())
}

/// With `reverify`, the provider also goes back to disabled, not required,
/// and unverified until root completes another test sign-in.
pub async fn update_provider(
  conn: &mut SqliteConnection,
  fields: ProviderFields<'_>,
  reverify: bool,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE sso_providers SET preset=?,display_name=?,issuer=?,client_id=?,client_secret_ciphertext=?,client_secret_nonce=?,enabled=CASE WHEN ? THEN 0 ELSE enabled END,require_sso=CASE WHEN ? THEN 0 ELSE require_sso END,verified_at=CASE WHEN ? THEN NULL ELSE verified_at END,updated_at=? WHERE id=?")
    .bind(fields.preset)
    .bind(fields.display_name)
    .bind(fields.issuer)
    .bind(fields.client_id)
    .bind(fields.client_secret_ciphertext)
    .bind(fields.client_secret_nonce)
    .bind(reverify)
    .bind(reverify)
    .bind(reverify)
    .bind(fields.now)
    .bind(fields.id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Turning the provider off also stops requiring it.
pub async fn set_enabled(
  conn: &mut SqliteConnection,
  id: &str,
  enabled: bool,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query(
    "UPDATE sso_providers SET enabled=?,require_sso=require_sso AND ?,updated_at=? WHERE id=?",
  )
  .bind(enabled)
  .bind(enabled)
  .bind(now)
  .bind(id)
  .execute(conn)
  .await?;
  Ok(())
}

pub async fn set_required(
  conn: &mut SqliteConnection,
  id: &str,
  required: bool,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE sso_providers SET require_sso=?,updated_at=? WHERE id=?")
    .bind(required)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

pub async fn set_password_form_hidden(
  conn: &mut SqliteConnection,
  id: &str,
  hidden: bool,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE sso_providers SET hide_password_form=?,updated_at=? WHERE id=?")
    .bind(hidden)
    .bind(now)
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Linked identities and pending sign-ins cascade with it.
pub async fn delete_provider(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM sso_providers WHERE id=?")
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Records root's first successful test sign-in. Returns whether this call
/// set it.
pub async fn mark_verified(
  conn: &mut SqliteConnection,
  id: &str,
  now: &str,
) -> Result<bool, sqlx::Error> {
  let updated =
    sqlx::query("UPDATE sso_providers SET verified_at=? WHERE id=? AND verified_at IS NULL")
      .bind(now)
      .bind(id)
      .execute(conn)
      .await?
      .rows_affected();
  Ok(updated == 1)
}

/// Signs out every account except root.
pub async fn revoke_non_root_sessions(
  conn: &mut SqliteConnection,
  now: &str,
) -> Result<u64, sqlx::Error> {
  // TODO(rbac): root is the fixed break-glass role. Exempt whichever roles
  // may keep password sign-in once roles are configurable.
  Ok(
    sqlx::query("UPDATE sessions SET revoked_at=? WHERE revoked_at IS NULL AND admin_id IN (SELECT id FROM admins WHERE is_root=0)")
      .bind(now)
      .execute(conn)
      .await?
      .rows_affected(),
  )
}

pub struct NewIdentity<'a> {
  pub id: &'a str,
  pub admin_id: &'a str,
  pub provider_id: &'a str,
  pub issuer: &'a str,
  pub subject: &'a str,
  pub email_at_link: Option<&'a str>,
  /// `auto` for a verified email match, `manual` for a link the account
  /// started itself.
  pub linked_via: &'a str,
  pub now: &'a str,
  /// Set when the link happened during a sign-in.
  pub last_login_at: Option<&'a str>,
}

/// Fails with a unique violation when the subject or the account is
/// already linked to this provider.
pub async fn insert_identity(
  conn: &mut SqliteConnection,
  identity: NewIdentity<'_>,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO admin_identities(id,admin_id,provider_id,issuer,subject,email_at_link,linked_via,created_at,last_login_at) VALUES(?,?,?,?,?,?,?,?,?)")
    .bind(identity.id)
    .bind(identity.admin_id)
    .bind(identity.provider_id)
    .bind(identity.issuer)
    .bind(identity.subject)
    .bind(identity.email_at_link)
    .bind(identity.linked_via)
    .bind(identity.now)
    .bind(identity.last_login_at)
    .execute(conn)
    .await?;
  Ok(())
}

/// The account an (issuer, subject) pair is linked to.
pub async fn identity_owner(
  conn: &mut SqliteConnection,
  issuer: &str,
  subject: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT admin_id FROM admin_identities WHERE issuer=? AND subject=?")
    .bind(issuer)
    .bind(subject)
    .fetch_optional(conn)
    .await
}

/// The ID and role of the account an (issuer, subject) pair is linked to.
pub async fn linked_account(
  conn: &mut SqliteConnection,
  issuer: &str,
  subject: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
  sqlx::query_as(
    "SELECT a.id,a.role FROM admin_identities i JOIN admins a ON a.id=i.admin_id WHERE i.issuer=? AND i.subject=?",
  )
  .bind(issuer)
  .bind(subject)
  .fetch_optional(conn)
  .await
}

pub async fn touch_identity(
  conn: &mut SqliteConnection,
  issuer: &str,
  subject: &str,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE admin_identities SET last_login_at=? WHERE issuer=? AND subject=?")
    .bind(now)
    .bind(issuer)
    .bind(subject)
    .execute(conn)
    .await?;
  Ok(())
}

/// The email at link time and the link time of `admin_id`'s identity.
pub async fn identity_of(
  pool: &SqlitePool,
  admin_id: &str,
  provider_id: &str,
) -> Result<Option<(Option<String>, String)>, sqlx::Error> {
  sqlx::query_as(
    "SELECT email_at_link,created_at FROM admin_identities WHERE admin_id=? AND provider_id=?",
  )
  .bind(admin_id)
  .bind(provider_id)
  .fetch_optional(pool)
  .await
}

/// Removes identities from any other issuer, which can never match a
/// sign-in again. Returns how many went.
pub async fn delete_identities_from_other_issuers(
  conn: &mut SqliteConnection,
  provider_id: &str,
  issuer: &str,
) -> Result<u64, sqlx::Error> {
  Ok(
    sqlx::query("DELETE FROM admin_identities WHERE provider_id=? AND issuer<>?")
      .bind(provider_id)
      .bind(issuer)
      .execute(conn)
      .await?
      .rows_affected(),
  )
}

pub async fn delete_identities_of(
  conn: &mut SqliteConnection,
  admin_id: &str,
) -> Result<u64, sqlx::Error> {
  Ok(
    sqlx::query("DELETE FROM admin_identities WHERE admin_id=?")
      .bind(admin_id)
      .execute(conn)
      .await?
      .rows_affected(),
  )
}

/// Finished and abandoned sign-ins are cleared on the recurring start path.
pub async fn delete_expired_flows(
  conn: &mut SqliteConnection,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM sso_flows WHERE expires_at < ?")
    .bind(now)
    .execute(conn)
    .await?;
  Ok(())
}

/// Sign-ins in progress overall and from `client_ip`.
pub async fn pending_flows(
  conn: &mut SqliteConnection,
  client_ip: Option<&str>,
) -> Result<(i64, i64), sqlx::Error> {
  sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(client_ip IS ?), 0) FROM sso_flows")
    .bind(client_ip)
    .fetch_one(conn)
    .await
}

pub struct NewFlow<'a> {
  pub id: &'a str,
  pub state_hash: &'a [u8],
  pub provider_id: &'a str,
  pub purpose: &'a str,
  pub nonce: &'a str,
  pub pkce_verifier: &'a str,
  pub binding_hash: &'a [u8],
  pub session_id: Option<&'a str>,
  pub admin_id: Option<&'a str>,
  pub redirect_to: Option<&'a str>,
  pub client_ip: Option<&'a str>,
  pub now: &'a str,
  pub expires_at: &'a str,
}

pub async fn insert_flow(
  conn: &mut SqliteConnection,
  flow: NewFlow<'_>,
) -> Result<(), sqlx::Error> {
  sqlx::query("INSERT INTO sso_flows(id,state_hash,provider_id,purpose,nonce,pkce_verifier,binding_hash,session_id,admin_id,redirect_to,client_ip,created_at,expires_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)")
    .bind(flow.id)
    .bind(flow.state_hash)
    .bind(flow.provider_id)
    .bind(flow.purpose)
    .bind(flow.nonce)
    .bind(flow.pkce_verifier)
    .bind(flow.binding_hash)
    .bind(flow.session_id)
    .bind(flow.admin_id)
    .bind(flow.redirect_to)
    .bind(flow.client_ip)
    .bind(flow.now)
    .bind(flow.expires_at)
    .execute(conn)
    .await?;
  Ok(())
}

#[derive(sqlx::FromRow)]
pub struct FlowRow {
  pub id: String,
  pub provider_id: String,
  pub purpose: String,
  pub nonce: String,
  pub pkce_verifier: String,
  pub session_id: Option<String>,
  pub admin_id: Option<String>,
  pub created_at: String,
}

/// Marks the flow for this state and browser binding used, in the same
/// statement that finds it, so two racing callbacks cannot both use it.
pub async fn consume_flow(
  pool: &SqlitePool,
  state_hash: &[u8],
  binding_hash: &[u8],
  now: &str,
) -> Result<Option<FlowRow>, sqlx::Error> {
  sqlx::query_as(
    "UPDATE sso_flows SET consumed_at=? WHERE state_hash=? AND binding_hash=? AND consumed_at IS NULL AND expires_at > ? RETURNING id,provider_id,purpose,nonce,pkce_verifier,session_id,admin_id,created_at",
  )
  .bind(now)
  .bind(state_hash)
  .bind(binding_hash)
  .bind(now)
  .fetch_optional(pool)
  .await
}

pub async fn set_handoff(
  conn: &mut SqliteConnection,
  flow_id: &str,
  handoff_hash: &[u8],
  expires_at: &str,
  admin_id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE sso_flows SET handoff_hash=?,handoff_expires_at=?,admin_id=? WHERE id=?")
    .bind(handoff_hash)
    .bind(expires_at)
    .bind(admin_id)
    .bind(flow_id)
    .execute(conn)
    .await?;
  Ok(())
}

/// Deletes the login flow holding this handoff code and browser binding,
/// returning its account and redirect. Used once, even under a race.
pub async fn take_handoff(
  conn: &mut SqliteConnection,
  handoff_hash: &[u8],
  binding_hash: &[u8],
  now: &str,
) -> Result<Option<(Option<String>, Option<String>)>, sqlx::Error> {
  sqlx::query_as(
    "DELETE FROM sso_flows WHERE handoff_hash=? AND binding_hash=? AND handoff_expires_at > ? AND purpose='login' RETURNING admin_id,redirect_to",
  )
  .bind(handoff_hash)
  .bind(binding_hash)
  .bind(now)
  .fetch_optional(conn)
  .await
}

pub async fn delete_flow(
  conn: &mut SqliteConnection,
  id: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("DELETE FROM sso_flows WHERE id=?")
    .bind(id)
    .execute(conn)
    .await?;
  Ok(())
}

/// The email and role of an account.
pub async fn account(
  conn: &mut SqliteConnection,
  admin_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
  sqlx::query_as("SELECT email,role FROM admins WHERE id=?")
    .bind(admin_id)
    .fetch_optional(conn)
    .await
}

/// The ID and role of the account with this email.
pub async fn account_by_email(
  conn: &mut SqliteConnection,
  email: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
  sqlx::query_as("SELECT id,role FROM admins WHERE email=? COLLATE NOCASE")
    .bind(email)
    .fetch_optional(conn)
    .await
}

pub async fn password_hash(
  pool: &SqlitePool,
  admin_id: &str,
) -> Result<Option<String>, sqlx::Error> {
  sqlx::query_scalar("SELECT password_hash FROM admins WHERE id=?")
    .bind(admin_id)
    .fetch_optional(pool)
    .await
}

/// The email and role of the account behind a session that is still live.
pub async fn live_session_account(
  conn: &mut SqliteConnection,
  session_id: Option<&str>,
  admin_id: &str,
  now: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
  sqlx::query_as(
    "SELECT a.email,a.role FROM sessions s JOIN admins a ON a.id=s.admin_id WHERE s.id=? AND s.admin_id=? AND s.revoked_at IS NULL AND s.idle_expires_at > ? AND s.absolute_expires_at > ?",
  )
  .bind(session_id)
  .bind(admin_id)
  .bind(now)
  .bind(now)
  .fetch_optional(conn)
  .await
}

pub async fn mark_recent_auth(
  conn: &mut SqliteConnection,
  session_id: Option<&str>,
  now: &str,
) -> Result<(), sqlx::Error> {
  sqlx::query("UPDATE sessions SET recent_auth_at=? WHERE id=?")
    .bind(now)
    .bind(session_id)
    .execute(conn)
    .await?;
  Ok(())
}
