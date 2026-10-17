use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
  config::ServerConfig,
  services::{
    cache::RateLimiter,
    crypto::CryptoService,
    db::DbClient,
    oidc::{OidcLogin, OidcVerifier},
  },
};

#[derive(Clone)]
pub struct AppState {
  pub config: Arc<ServerConfig>,
  pub db: DbClient,
  pub crypto: CryptoService,
  /// Verifies OIDC assertions when the server has trusted providers configured.
  pub oidc: Option<OidcVerifier>,
  /// Browser sign-in configuration, when an operator enabled it.
  pub oidc_login: Option<OidcLogin>,
  pub setup: Arc<RwLock<SetupState>>,
  pub rate_limiter: RateLimiter,
  pub maintenance: Arc<std::sync::atomic::AtomicBool>,
  pub started_at: std::time::Instant,
}

#[derive(Default)]
pub struct SetupState {
  pub token: Option<String>,
}
