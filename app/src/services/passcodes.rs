//! One-time passcodes tied to an account and a purpose, stored in the
//! `passcodes` table. Only a hash of the purpose and the code is kept, so a
//! code issued for one purpose never matches a lookup for another. An
//! account holds at most one live code per purpose: issuing again replaces
//! it.
//!
//! Nothing here runs on a timer. Callers sweep expired rows with
//! [`sweep_expired`] when they meet an expired code or a failed attempt.

use crate::{
  constants::{
    limits::{
      LAUNCH_PASSCODE_DEFAULT_SECONDS, LAUNCH_PASSCODE_MAX_FAILURES, LAUNCH_PASSCODE_MAX_SECONDS,
      LAUNCH_PASSCODE_MIN_SECONDS,
    },
    tokens::PASSCODE_ID_PREFIX,
  },
  services::token,
};
use chrono::{DateTime, Duration, Utc};
use sqlx::SqliteConnection;
use std::ops::RangeInclusive;

/// What a passcode is for. The database stores [`PasscodePurpose::as_str`];
/// this enum is the list of purposes the server accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasscodePurpose {
  /// `dopbase admin passcode`: sign an account in to the Admin UI from the
  /// server machine.
  Launch,
}

impl PasscodePurpose {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Launch => "launch",
    }
  }

  /// Lifetime when the caller does not choose one.
  pub fn default_ttl_seconds(self) -> i64 {
    match self {
      Self::Launch => LAUNCH_PASSCODE_DEFAULT_SECONDS,
    }
  }

  /// Lifetimes a caller may choose.
  pub fn ttl_range_seconds(self) -> RangeInclusive<i64> {
    match self {
      Self::Launch => LAUNCH_PASSCODE_MIN_SECONDS..=LAUNCH_PASSCODE_MAX_SECONDS,
    }
  }

  /// Wrong tries that burn the code.
  pub fn max_failures(self) -> i64 {
    match self {
      Self::Launch => LAUNCH_PASSCODE_MAX_FAILURES,
    }
  }
}

/// What happened to a presented code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consumed {
  /// The code matched and was live. Its row is gone, so it cannot be used
  /// again.
  Used,
  /// The account has a live code and this was not it.
  Wrong,
  /// This wrong try was the last one allowed; the code is gone.
  Burned,
  /// The code had expired; its row is gone.
  Expired,
  /// The account has no code for this purpose.
  Missing,
}

/// The stored hash. Binding the purpose in keeps codes of different
/// purposes apart even if a query forgets to filter on it.
fn code_hash(
  purpose: PasscodePurpose,
  code: &str,
) -> Vec<u8> {
  token::hash(&format!("{}:{code}", purpose.as_str()))
}

fn timestamp(time: DateTime<Utc>) -> String {
  time.to_rfc3339()
}

/// Store `code` (canonical form, see `utils::passcode::normalize`) as the
/// account's live passcode for `purpose`, replacing any earlier one.
/// Returns when it expires. Run it inside the caller's transaction.
pub async fn issue(
  connection: &mut SqliteConnection,
  purpose: PasscodePurpose,
  admin_id: &str,
  code: &str,
  ttl: Duration,
  issued_by: Option<&str>,
  now: DateTime<Utc>,
) -> Result<DateTime<Utc>, sqlx::Error> {
  let expires_at = now + ttl;
  sqlx::query(
    "INSERT INTO passcodes(id,purpose,admin_id,code_hash,failed_attempts,issued_by,created_at,expires_at) \
     VALUES(?,?,?,?,0,?,?,?) \
     ON CONFLICT(purpose,admin_id) DO UPDATE SET id=excluded.id,code_hash=excluded.code_hash,\
     failed_attempts=0,issued_by=excluded.issued_by,created_at=excluded.created_at,expires_at=excluded.expires_at",
  )
  .bind(token::public_id(PASSCODE_ID_PREFIX))
  .bind(purpose.as_str())
  .bind(admin_id)
  .bind(code_hash(purpose, code))
  .bind(issued_by)
  .bind(timestamp(now))
  .bind(timestamp(expires_at))
  .execute(&mut *connection)
  .await?;
  Ok(expires_at)
}

/// Check a presented code and use it up when it matches. `code` is the
/// canonical form, or `None` when what was typed cannot be a code, which
/// still counts as a wrong try. Run it inside a transaction that already
/// holds the write lock (`BEGIN IMMEDIATE`): the steps read before they
/// write, and two requests presenting the same code must not both see
/// [`Consumed::Used`].
pub async fn consume(
  connection: &mut SqliteConnection,
  purpose: PasscodePurpose,
  admin_id: &str,
  code: Option<&str>,
  now: DateTime<Utc>,
) -> Result<Consumed, sqlx::Error> {
  let now_text = timestamp(now);
  if let Some(code) = code {
    let used: Option<String> = sqlx::query_scalar(
      "DELETE FROM passcodes WHERE purpose=? AND admin_id=? AND code_hash=? AND expires_at > ? RETURNING id",
    )
    .bind(purpose.as_str())
    .bind(admin_id)
    .bind(code_hash(purpose, code))
    .bind(&now_text)
    .fetch_optional(&mut *connection)
    .await?;
    if used.is_some() {
      return Ok(Consumed::Used);
    }
  }

  let live: Option<(String, i64, String)> = sqlx::query_as(
    "SELECT id,failed_attempts,expires_at FROM passcodes WHERE purpose=? AND admin_id=?",
  )
  .bind(purpose.as_str())
  .bind(admin_id)
  .fetch_optional(&mut *connection)
  .await?;
  let Some((id, failed_attempts, expires_at)) = live else {
    return Ok(Consumed::Missing);
  };
  if expires_at <= now_text {
    sqlx::query("DELETE FROM passcodes WHERE id=?")
      .bind(&id)
      .execute(&mut *connection)
      .await?;
    return Ok(Consumed::Expired);
  }
  if failed_attempts + 1 >= purpose.max_failures() {
    sqlx::query("DELETE FROM passcodes WHERE id=?")
      .bind(&id)
      .execute(&mut *connection)
      .await?;
    return Ok(Consumed::Burned);
  }
  sqlx::query("UPDATE passcodes SET failed_attempts=failed_attempts+1 WHERE id=?")
    .bind(&id)
    .execute(&mut *connection)
    .await?;
  Ok(Consumed::Wrong)
}

/// Delete every expired passcode, whatever its purpose. Returns how many
/// rows went.
pub async fn sweep_expired(
  connection: &mut SqliteConnection,
  now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
  Ok(
    sqlx::query("DELETE FROM passcodes WHERE expires_at <= ?")
      .bind(timestamp(now))
      .execute(&mut *connection)
      .await?
      .rows_affected(),
  )
}
