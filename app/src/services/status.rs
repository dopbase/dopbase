//! Cached results of the health checks that are too slow to run on every
//! Status page poll.
//!
//! The page polls every 30 seconds. Walking the audit hash chain reads every
//! record, so a result is kept for a while and reused. Nothing here runs on
//! a timer: work only happens when someone opens the Status page.

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

/// How old a cached audit chain result may get before a read verifies the
/// chain again.
pub const AUDIT_CHECK_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// The shortest gap between two runs someone asks for with Run checks.
pub const FORCED_RUN_INTERVAL: Duration = Duration::from_secs(10);

/// How often GitHub is asked for the latest release, when the opt-in
/// `update_check` setting is on.
pub const RELEASE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// How long to wait after a failed release lookup before trying again.
pub const RELEASE_RETRY_INTERVAL: Duration = Duration::from_secs(60 * 60);

const RELEASE_API_URL: &str = "https://api.github.com/repos/dopbase/dopbase/releases/latest";

/// One audit chain verification.
#[derive(Clone, Debug)]
pub struct AuditCheck {
  /// `Err` holds a short reason when verification could not run.
  pub outcome: Result<AuditChainOutcome, String>,
  pub checked_at: DateTime<Utc>,
  ran: Instant,
}

#[derive(Clone, Debug)]
pub struct AuditChainOutcome {
  pub intact: bool,
  /// Records walked before the first break, or all of them.
  pub records: u64,
  /// Sequence number of the first broken record.
  pub broken_seq: Option<i64>,
}

/// The latest published release, as last seen on GitHub.
#[derive(Clone, Debug)]
pub struct ReleaseCheck {
  /// Tag of the latest release, such as `v0.3.1`.
  pub latest: String,
  pub update_available: bool,
  pub release_url: String,
  pub checked_at: DateTime<Utc>,
}

/// What the Status page knows about newer releases.
#[derive(Clone, Debug, Default)]
pub struct ReleaseState {
  /// The last successful lookup. A failed lookup never clears it.
  pub last: Option<ReleaseCheck>,
  /// Whether the most recent lookup failed.
  pub failed: bool,
}

#[derive(Clone)]
pub struct StatusMonitor {
  inner: Arc<Mutex<State>>,
  release_url: Arc<std::sync::RwLock<String>>,
}

impl Default for StatusMonitor {
  fn default() -> Self {
    Self {
      inner: Arc::default(),
      release_url: Arc::new(std::sync::RwLock::new(RELEASE_API_URL.to_owned())),
    }
  }
}

#[derive(Default)]
struct State {
  audit: Option<AuditCheck>,
  last_forced: Option<Instant>,
  release: ReleaseState,
  release_attempted: Option<Instant>,
}

impl StatusMonitor {
  /// The cached audit chain result, verified now when there is none or it
  /// is older than [`AUDIT_CHECK_INTERVAL`].
  pub async fn audit(
    &self,
    pool: &SqlitePool,
  ) -> AuditCheck {
    let mut state = self.inner.lock().await;
    match &state.audit {
      Some(check) if check.ran.elapsed() < AUDIT_CHECK_INTERVAL => check.clone(),
      _ => {
        let check = verify(pool).await;
        state.audit = Some(check.clone());
        check
      }
    }
  }

  /// Verify the audit chain now and keep the result.
  pub async fn refresh(
    &self,
    pool: &SqlitePool,
  ) -> AuditCheck {
    let mut state = self.inner.lock().await;
    let check = verify(pool).await;
    state.audit = Some(check.clone());
    check
  }

  /// Claim a forced run. False when the last one was under
  /// [`FORCED_RUN_INTERVAL`] ago.
  pub async fn claim_forced_run(&self) -> bool {
    let mut state = self.inner.lock().await;
    if state
      .last_forced
      .is_some_and(|last| last.elapsed() < FORCED_RUN_INTERVAL)
    {
      return false;
    }
    state.last_forced = Some(Instant::now());
    true
  }

  /// The cached release lookup. Never calls GitHub.
  pub async fn release(&self) -> ReleaseState {
    self.inner.lock().await.release.clone()
  }

  /// Point release lookups at another server, such as a mirror or a test
  /// double.
  pub fn set_release_url(
    &self,
    url: impl Into<String>,
  ) {
    *self.release_url.write().unwrap() = url.into();
  }

  /// Look up the latest release unless the last success is under a day old
  /// or the last failure under an hour old. A due lookup is claimed before
  /// GitHub is asked, so overlapping calls make only one request.
  pub async fn refresh_release_if_due(&self) {
    let due = {
      let mut state = self.inner.lock().await;
      let wait = if state.release.failed {
        RELEASE_RETRY_INTERVAL
      } else {
        RELEASE_CHECK_INTERVAL
      };
      let due = state
        .release_attempted
        .is_none_or(|attempted| attempted.elapsed() >= wait);
      if due {
        state.release_attempted = Some(Instant::now());
      }
      due
    };
    if due {
      self.refresh_release().await;
    }
  }

  /// Look up the latest release now. A failure keeps the last result.
  pub async fn refresh_release(&self) {
    let url = self.release_url.read().unwrap().clone();
    let result = crate::cli::commands::update::check_release(env!("CARGO_PKG_VERSION"), &url).await;
    let mut state = self.inner.lock().await;
    state.release_attempted = Some(Instant::now());
    match result {
      Ok(status) => {
        state.release = ReleaseState {
          last: Some(ReleaseCheck {
            latest: status.latest_version,
            update_available: status.update_available,
            release_url: status.release_url,
            checked_at: Utc::now(),
          }),
          failed: false,
        };
      }
      Err(error) => {
        tracing::warn!(error = format!("{error:#}"), "latest release lookup failed");
        state.release.failed = true;
      }
    }
  }
}

async fn verify(pool: &SqlitePool) -> AuditCheck {
  let outcome = crate::services::audit_chain::verify(pool)
    .await
    .map(|report| AuditChainOutcome {
      intact: report.ok,
      records: report.records,
      broken_seq: report.broken.map(|broken| broken.seq),
    })
    .map_err(|error| {
      tracing::warn!(%error, "audit chain verification could not run");
      "The audit log could not be read.".to_owned()
    });
  AuditCheck {
    outcome,
    checked_at: Utc::now(),
    ran: Instant::now(),
  }
}
