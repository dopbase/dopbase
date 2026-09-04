use std::{
  fs::{File, OpenOptions},
  sync::Arc,
  time::Duration,
};

use anyhow::{Context, Result};
use axum::{
  Router,
  extract::DefaultBodyLimit,
  http::{HeaderName, StatusCode, Uri},
  middleware,
  response::{IntoResponse, Response},
};
use fs2::FileExt;
use tower_http::{
  request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
  timeout::TimeoutLayer,
  trace::TraceLayer,
};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
  config::{ServerConfig, database_path, ensure_data_dir},
  constants::errors::{INTERNAL_ERROR, REQUEST_INVALID},
  http::HttpError,
  modules,
  services::{cache::RateLimiter, crypto::CryptoService, db::DbClient, token},
  state::{AppState, SetupState},
};

pub async fn build_state(config: ServerConfig) -> Result<AppState> {
  ensure_data_dir(&config.data_dir)?;
  let db = DbClient::connect(&config.database_url)
    .await
    .context("failed to open SQLite")?;
  db.migrate()
    .await
    .context("failed to run database migrations")?;
  let crypto = CryptoService::initialize(db.pool(), &config.master_key.path)
    .await
    .context("failed to initialize master key")?;
  let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
    .fetch_one(db.pool())
    .await?;
  let setup_token = if admin_count == 0 {
    Some(token::generate("setup_")?)
  } else {
    None
  };
  Ok(AppState {
    config: Arc::new(config),
    db,
    crypto,
    setup: Arc::new(tokio::sync::RwLock::new(SetupState { token: setup_token })),
    rate_limiter: RateLimiter::default(),
  })
}

pub fn router(state: AppState) -> Router {
  let request_id = HeaderName::from_static("x-request-id");
  let mut router = Router::new().merge(modules::routes());
  if state.config.docs_enabled {
    let mut openapi = modules::openapi();
    let components = openapi.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
      "bearerAuth",
      SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
    );
    components.add_security_scheme(
      "cookieAuth",
      SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("dopbase_session"))),
    );
    components.add_security_scheme(
      "csrfHeader",
      SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Dopbase-CSRF"))),
    );
    router = router.merge(SwaggerUi::new("/api/docs").url("/api/v1/openapi.json", openapi));
  }
  router
    .fallback(static_fallback)
    .layer(DefaultBodyLimit::max(3 * 1024 * 1024))
    .layer(TimeoutLayer::with_status_code(
      StatusCode::REQUEST_TIMEOUT,
      Duration::from_secs(30),
    ))
    .layer(TraceLayer::new_for_http())
    .layer(PropagateRequestIdLayer::new(request_id.clone()))
    .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
    .layer(middleware::map_response(normalize_error_response))
    .with_state(state)
}

async fn normalize_error_response(response: Response) -> Response {
  let status = response.status();
  if !status.is_client_error() && !status.is_server_error() {
    return response;
  }
  let is_json = response
    .headers()
    .get(axum::http::header::CONTENT_TYPE)
    .and_then(|value| value.to_str().ok())
    .is_some_and(|value| value.starts_with("application/json"));
  if is_json {
    return response;
  }
  let request_id = response.headers().get("x-request-id").cloned();
  let error = if status.is_server_error() {
    HttpError::new(status, INTERNAL_ERROR, "An internal error occurred.")
  } else {
    HttpError::new(
      status,
      REQUEST_INVALID,
      "The request could not be processed.",
    )
  };
  let mut normalized = error.into_response();
  if let Some(request_id) = request_id {
    normalized.headers_mut().insert("x-request-id", request_id);
  }
  normalized
}

pub async fn serve(config: ServerConfig) -> Result<()> {
  serve_with_ready(config, None).await
}

pub fn startup_banner(
  public_url: &str,
  data_dir: &std::path::Path,
  docs_enabled: bool,
) -> String {
  let public_url = public_url.trim_end_matches('/');
  let mut rows = vec![
    "Dopbase".to_string(),
    "Secure, Simple and Private".to_string(),
    format!("Version {}", env!("CARGO_PKG_VERSION")),
    String::new(),
    format!("Admin UI:   {public_url}"),
    format!("API:        {public_url}/api/v1"),
    format!("Config:     {}", data_dir.display()),
  ];
  if docs_enabled {
    rows.push(format!("Swagger:    {public_url}/api/docs"));
  }
  let width = rows
    .iter()
    .map(|row| row.chars().count())
    .max()
    .unwrap_or(0)
    .max(62);
  let border = "─".repeat(width + 4);
  let mut lines = Vec::with_capacity(rows.len() + 2);
  lines.push(format!("╭{border}╮"));
  for row in rows {
    lines.push(format!("│  {row:<width$}  │"));
  }
  lines.push(format!("╰{border}╯"));
  lines.join("\n")
}

/// Run the server, optionally reporting readiness (or the startup failure) to
/// the foreground command that spawned it in background mode.
pub async fn serve_with_ready(
  config: ServerConfig,
  ready: Option<&crate::daemon::Ready>,
) -> Result<()> {
  ensure_data_dir(&config.data_dir)?;
  let _lock = InstanceLock::acquire(&config.database_url)?;
  let state = build_state(config).await?;
  let setup_token = state.setup.read().await.token.clone();
  let address = state.config.bind_addr()?;
  let grace = state.config.shutdown_grace_seconds;
  let listener = tokio::net::TcpListener::bind(address)
    .await
    .with_context(|| format!("failed to bind {address}"))?;
  let daemonized = state.config.daemonized;
  let _pid_file_lock = if daemonized {
    Some(crate::daemon::write_pid_file(
      &crate::daemon::pid_file_path(&state.config.data_dir),
      std::process::id(),
      &state.config.bind_address,
      &state.config.public_url,
    )?)
  } else {
    None
  };
  if let Some(ready) = ready {
    ready.ok(std::process::id(), setup_token.as_deref());
  }
  let public_url = state.config.public_url.trim_end_matches('/');
  eprintln!(
    "\n{}\n",
    startup_banner(
      public_url,
      &state.config.data_dir,
      state.config.docs_enabled
    )
  );
  if let Some(setup) = setup_token.as_deref() {
    eprintln!("\nDopbase setup token (shown once):\n{setup}\n");
  }
  tracing::info!(%address,"Dopbase server started");
  let serve_result = axum::serve(listener, router(state.clone()))
    .with_graceful_shutdown(shutdown_signal())
    .await;
  if daemonized {
    let _ = crate::daemon::remove_pid_file(&crate::daemon::pid_file_path(&state.config.data_dir));
  }
  serve_result?;
  if let Err(error) = state.db.checkpoint().await {
    tracing::warn!(%error, "failed to checkpoint SQLite WAL during shutdown");
  }
  let _ = tokio::time::timeout(Duration::from_secs(grace), state.db.close()).await;
  Ok(())
}

async fn shutdown_signal() {
  let ctrl_c = async {
    let _ = tokio::signal::ctrl_c().await;
  };
  #[cfg(unix)]
  let terminate = async {
    if let Ok(mut signal) =
      tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
      signal.recv().await;
    } else {
      std::future::pending::<()>().await;
    }
  };
  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();
  tokio::select! {_=ctrl_c=>{},_=terminate=>{}}
}

pub struct InstanceLock {
  file: File,
}
impl InstanceLock {
  pub fn acquire(database_url: &str) -> Result<Option<Self>> {
    if database_url.contains(":memory:") {
      return Ok(None);
    }
    let file = Self::open(database_url)?;
    file
      .try_lock_exclusive()
      .context(
        "Dopbase server is already running for this database. \nStop the running server before starting another one",
      )?;
    Ok(Some(Self { file }))
  }

  pub fn is_held(database_url: &str) -> Result<bool> {
    if database_url.contains(":memory:") {
      return Ok(false);
    }
    let file = Self::open(database_url)?;
    match file.try_lock_exclusive() {
      Ok(()) => {
        file.unlock()?;
        Ok(false)
      }
      Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
      Err(error) => Err(error).context("failed to inspect the Dopbase database lock"),
    }
  }

  fn open(database_url: &str) -> Result<File> {
    let database = database_path(database_url)?;
    let lock = std::path::PathBuf::from(format!("{}.lock", database.display()));
    if let Some(parent) = lock.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
      .create(true)
      .truncate(false)
      .read(true)
      .write(true)
      .open(&lock)?;
    Ok(file)
  }
}
impl Drop for InstanceLock {
  fn drop(&mut self) {
    let _ = self.file.unlock();
  }
}

async fn static_fallback(uri: Uri) -> Response {
  if uri.path().starts_with("/api/") {
    return HttpError::not_found(REQUEST_INVALID, "The requested API route was not found.")
      .into_response();
  }
  embedded_asset(uri.path()).unwrap_or_else(|| {
    // Asset-looking paths must 404 instead of falling back to
    // index.html — serving HTML for a missing .js/.css masks broken
    // asset URLs as a silently blank page ("Failed to load module
    // script" with an HTML MIME type).
    if looks_like_asset(uri.path()) {
      return StatusCode::NOT_FOUND.into_response();
    }
    embedded_asset("/index.html").unwrap_or_else(|| StatusCode::NOT_FOUND.into_response())
  })
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../dist/"]
struct AdminAssets;

fn looks_like_asset(path: &str) -> bool {
  const ASSET_EXTENSIONS: [&str; 12] = [
    "js", "mjs", "css", "map", "woff", "woff2", "ttf", "otf", "svg", "png", "jpg", "ico",
  ];
  let name = path.rsplit('/').next().unwrap_or(path);
  name
    .split_once('.')
    .is_some_and(|(_, ext)| ASSET_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn embedded_asset(path: &str) -> Option<Response> {
  use axum::http::{HeaderValue, header};
  let key = path.trim_start_matches('/');
  let key = if key.is_empty() { "index.html" } else { key };
  let asset = AdminAssets::get(key)?;
  let mime = mime_guess::from_path(key).first_or_octet_stream();
  let mut response = asset.data.into_owned().into_response();
  response.headers_mut().insert(
    header::CONTENT_TYPE,
    HeaderValue::from_str(mime.as_ref()).ok()?,
  );
  let cache = if key == "index.html" {
    "no-cache"
  } else {
    "public, max-age=31536000, immutable"
  };
  response
    .headers_mut()
    .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
  Some(response)
}
