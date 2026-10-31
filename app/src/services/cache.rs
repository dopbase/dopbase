use std::{
  collections::HashMap,
  sync::Arc,
  time::{Duration, Instant},
};
use tokio::sync::Mutex;

const WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_FAILURES: u32 = 5;
const MAX_ENTRIES: usize = 10_000;
// Only locked buckets can push the map past MAX_ENTRIES. Past this ceiling
// keys without a bucket are refused, so a flood cannot grow the map forever.
const HARD_MAX_ENTRIES: usize = 100_000;

#[derive(Clone)]
pub struct RateLimiter {
  entries: Arc<Mutex<HashMap<String, Entry>>>,
  max_entries: usize,
  hard_max_entries: usize,
}

struct Entry {
  failures: u32,
  started: Instant,
}

impl Entry {
  fn expired(&self) -> bool {
    self.started.elapsed() >= WINDOW
  }

  fn locked(&self) -> bool {
    !self.expired() && self.failures >= MAX_FAILURES
  }
}

impl Default for RateLimiter {
  fn default() -> Self {
    Self::with_capacity(MAX_ENTRIES, HARD_MAX_ENTRIES)
  }
}

impl RateLimiter {
  /// A limiter that starts evicting at `max_entries` buckets and refuses keys
  /// it has no bucket for once `hard_max_entries` are held.
  pub fn with_capacity(
    max_entries: usize,
    hard_max_entries: usize,
  ) -> Self {
    let max_entries = max_entries.max(1);
    Self {
      entries: Arc::default(),
      max_entries,
      hard_max_entries: hard_max_entries.max(max_entries),
    }
  }

  /// Free space for a new bucket once the map is full. Callers pick the keys,
  /// so eviction must not hand a budget back: locked buckets are never
  /// evicted, and the rest go fewest failures first, then oldest. Resetting a
  /// bucket with k failures means filling the map with buckets that each have
  /// at least k. Eviction removes a tenth of the cap at a time, so the full
  /// scan runs once per batch, not on every request.
  fn make_room(
    &self,
    entries: &mut HashMap<String, Entry>,
  ) {
    if entries.len() < self.max_entries {
      return;
    }
    entries.retain(|_, entry| !entry.expired());
    if entries.len() < self.max_entries {
      return;
    }
    let mut candidates: Vec<(u32, Instant, &String)> = entries
      .iter()
      .filter(|(_, entry)| !entry.locked())
      .map(|(key, entry)| (entry.failures, entry.started, key))
      .collect();
    let batch = (self.max_entries / 10).max(1).min(candidates.len());
    if batch == 0 {
      return;
    }
    candidates
      .select_nth_unstable_by_key(batch - 1, |(failures, started, _)| (*failures, *started));
    let evicted: Vec<String> = candidates[..batch]
      .iter()
      .map(|(_, _, key)| (*key).clone())
      .collect();
    for key in evicted {
      entries.remove(&key);
    }
  }

  pub async fn check(
    &self,
    key: &str,
  ) -> bool {
    let mut entries = self.entries.lock().await;
    if let Some(entry) = entries.get(key) {
      return !entry.locked();
    }
    if entries.len() >= self.hard_max_entries {
      entries.retain(|_, entry| !entry.expired());
    }
    entries.len() < self.hard_max_entries
  }

  pub async fn failure(
    &self,
    key: &str,
  ) {
    let mut entries = self.entries.lock().await;
    if !entries.contains_key(key) {
      self.make_room(&mut entries);
    }
    let entry = entries.entry(key.to_owned()).or_insert(Entry {
      failures: 0,
      started: Instant::now(),
    });
    if entry.expired() {
      *entry = Entry {
        failures: 0,
        started: Instant::now(),
      };
    }
    entry.failures += 1;
  }

  /// How long a key's failures count, which is also the longest a locked
  /// key waits.
  pub const fn window() -> Duration {
    WINDOW
  }

  /// Failures `key` may still have before `check` refuses it.
  pub async fn remaining(
    &self,
    key: &str,
  ) -> u32 {
    match self.entries.lock().await.get(key) {
      Some(entry) if !entry.expired() => MAX_FAILURES.saturating_sub(entry.failures),
      _ => MAX_FAILURES,
    }
  }

  /// How long until `check` lets a locked `key` through again; `None` when
  /// it is not locked.
  pub async fn retry_after(
    &self,
    key: &str,
  ) -> Option<Duration> {
    let entries = self.entries.lock().await;
    let entry = entries.get(key).filter(|entry| entry.locked())?;
    Some(WINDOW.saturating_sub(entry.started.elapsed()))
  }

  pub async fn clear(
    &self,
    key: &str,
  ) {
    self.entries.lock().await.remove(key);
  }
}
