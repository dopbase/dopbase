use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
  config::ServerConfig,
  services::{cache::RateLimiter, crypto::CryptoService, db::DbClient},
};

#[derive(Clone)]
pub struct AppState {
  pub config: Arc<ServerConfig>,
  pub db: DbClient,
  pub crypto: CryptoService,
  pub setup: Arc<RwLock<SetupState>>,
  pub rate_limiter: RateLimiter,
  pub maintenance: Arc<std::sync::atomic::AtomicBool>,
  pub started_at: std::time::Instant,
}

#[derive(Default)]
pub struct SetupState {
  pub token: Option<String>,
}
