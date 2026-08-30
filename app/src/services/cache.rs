use std::{
  collections::HashMap,
  sync::Arc,
  time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct RateLimiter {
  entries: Arc<Mutex<HashMap<String, Entry>>>,
}

struct Entry {
  failures: u32,
  started: Instant,
}

impl RateLimiter {
  pub async fn check(
    &self,
    key: &str,
  ) -> bool {
    let mut entries = self.entries.lock().await;
    let entry = entries.entry(key.to_owned()).or_insert(Entry {
      failures: 0,
      started: Instant::now(),
    });
    if entry.started.elapsed() >= Duration::from_secs(15 * 60) {
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
