use std::{
  collections::HashMap,
  sync::Arc,
  time::{Duration, Instant},
};
use tokio::sync::Mutex;

const WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_ENTRIES: usize = 10_000;

#[derive(Clone, Default)]
pub struct RateLimiter {
  entries: Arc<Mutex<HashMap<String, Entry>>>,
}

struct Entry {
  failures: u32,
  started: Instant,
}

impl RateLimiter {
  fn evict(
    entries: &mut HashMap<String, Entry>,
    requested_key: &str,
  ) {
    entries.retain(|_, entry| entry.started.elapsed() < WINDOW);
    if !entries.contains_key(requested_key)
      && entries.len() >= MAX_ENTRIES
      && let Some(oldest) = entries
        .iter()
        .max_by_key(|(_, entry)| entry.started.elapsed())
        .map(|(key, _)| key.clone())
    {
      entries.remove(&oldest);
    }
  }

  pub async fn check(
    &self,
    key: &str,
  ) -> bool {
    let mut entries = self.entries.lock().await;
    Self::evict(&mut entries, key);
    let entry = entries.entry(key.to_owned()).or_insert(Entry {
      failures: 0,
      started: Instant::now(),
    });
    if entry.started.elapsed() >= WINDOW {
      *entry = Entry {
        failures: 0,
        started: Instant::now(),
      };
    }
    entry.failures < 5
  }

  pub async fn failure(
    &self,
    key: &str,
  ) {
    let mut entries = self.entries.lock().await;
    Self::evict(&mut entries, key);
    let entry = entries.entry(key.to_owned()).or_insert(Entry {
      failures: 0,
      started: Instant::now(),
    });
    entry.failures += 1;
  }

  pub async fn clear(
    &self,
    key: &str,
  ) {
    self.entries.lock().await.remove(key);
  }
}
