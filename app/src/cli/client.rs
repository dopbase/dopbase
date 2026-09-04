use super::{local_config::ResolvedServer, session};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
  env, fmt,
  io::{self, IsTerminal, Write},
  time::Duration,
};

#[derive(Debug)]
pub enum CliCancelled {
  Login,
  PasswordConfirmation,
  ServerSwitch,
}

impl fmt::Display for CliCancelled {
  fn fmt(
    &self,
    formatter: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    formatter.write_str(match self {
      Self::Login => "Login cancelled.",
      Self::PasswordConfirmation => "Password confirmation cancelled.",
      Self::ServerSwitch => "Server switch cancelled.",
    })
  }
}

impl std::error::Error for CliCancelled {}

#[derive(Debug)]
pub(crate) struct AvailabilityError {
  message: String,
}

impl AvailabilityError {
  pub(crate) fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

impl fmt::Display for AvailabilityError {
  fn fmt(
    &self,
    formatter: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl std::error::Error for AvailabilityError {}

pub(crate) fn is_availability_error(error: &anyhow::Error) -> bool {
  error.downcast_ref::<AvailabilityError>().is_some()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
  Environment,
  EncryptedSession,
  None,
}

impl CredentialSource {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Environment => "environment",
      Self::EncryptedSession => "encrypted_session",
      Self::None => "none",
    }
  }
}

pub struct Credential {
  pub token: Option<String>,
  pub source: CredentialSource,
  pub email: Option<String>,
}

pub struct ApiClient {
  pub base_url: String,
  client: reqwest::Client,
  token: Option<String>,
}

const MAX_RUNTIME_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

impl ApiClient {
  pub fn new(
    server: &ResolvedServer,
    token: Option<String>,
  ) -> Result<Self> {
    Ok(Self {
      base_url: server.url.clone(),
      client: reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?,
      token,
    })
  }
  pub async fn request(
    &self,
    method: Method,
    path: &str,
    body: Option<Value>,
  ) -> Result<Value> {
    self.request_inner(method, path, body, false).await
  }

  pub(crate) async fn request_runtime(
    &self,
    method: Method,
    path: &str,
    body: Option<Value>,
  ) -> Result<Value> {
    self.request_inner(method, path, body, true).await
  }

  async fn request_inner(
    &self,
    method: Method,
    path: &str,
    body: Option<Value>,
    classify_availability: bool,
  ) -> Result<Value> {
    let mut request = self
      .client
      .request(method, format!("{}{}", self.base_url, path));
    if let Some(token) = &self.token {
      request = request.bearer_auth(token);
    }
    if let Some(body) = body {
      request = request.json(&body);
    }
    let response = request
      .send()
      .await
      .map_err(|error| transport_error(error, &self.base_url, classify_availability))?;
    let status = response.status();
    if classify_availability && status.is_server_error() {
      return Err(
        AvailabilityError::new(format!("Dopbase at {} returned {status}", self.base_url)).into(),
      );
    }
    let value: Value = if classify_availability {
      let bytes = read_limited_response(response, MAX_RUNTIME_RESPONSE_BYTES).await?;
      serde_json::from_slice(&bytes).context("server returned an invalid JSON response")?
    } else {
      response
        .json()
        .await
        .context("server returned an invalid JSON response")?
    };
    if !status.is_success() {
      let errors = value
        .get("error")
        .and_then(Value::as_object)
        .map(|errors| {
          errors
            .iter()
            .map(|(code, message)| {
              format!("{code}: {}", message.as_str().unwrap_or("Request failed."))
            })
            .collect::<Vec<_>>()
            .join("; ")
        })
        .unwrap_or_else(|| format!("server returned {status}"));
      bail!(errors);
    }
    Ok(value.get("data").cloned().unwrap_or(Value::Null))
  }
  pub async fn health(&self) -> Result<Value> {
    self.request(Method::GET, "/api/v1/health", None).await
  }

  pub(crate) fn credential_token(&self) -> Result<&str> {
    self
      .token
      .as_deref()
      .context("Dopbase authentication is required")
  }
}

fn transport_error(
  error: reqwest::Error,
  base_url: &str,
  classify_availability: bool,
) -> anyhow::Error {
  let message = if error.is_timeout() {
    format!(
      "Dopbase at {base_url} did not respond within 30 seconds.\n\
Check the server health and network connection, then try again."
    )
  } else if error.is_connect() {
    format!(
      "Could not connect to Dopbase at {base_url}.\n\
Check that the server is running and verify the active endpoint with `dopbase status`. For local development, start it with `dopbase serve`."
    )
  } else {
    format!(
      "The request to Dopbase at {base_url} failed before a response was received.\n\
Check DNS, TLS, proxy, and network settings, then try again."
    )
  };
  if classify_availability {
    return AvailabilityError::new(message).into();
  }
  anyhow::anyhow!(message)
}

async fn read_limited_response(
  mut response: reqwest::Response,
  limit: u64,
) -> Result<Vec<u8>> {
  if response
    .content_length()
    .is_some_and(|length| length > limit)
  {
    bail!("server runtime response exceeded the maximum allowed size");
  }
  let mut body = Vec::new();
  while let Some(chunk) = response.chunk().await? {
    if body.len() as u64 + chunk.len() as u64 > limit {
      bail!("server runtime response exceeded the maximum allowed size");
    }
    body.extend_from_slice(&chunk);
  }
  Ok(body)
}
pub fn credential(server: &ResolvedServer) -> Result<Credential> {
  if let Ok(token) = env::var("DOPBASE_TOKEN") {
    if token.is_empty() {
      bail!("DOPBASE_TOKEN is set but empty");
    }
    return Ok(Credential {
      token: Some(token),
      source: CredentialSource::Environment,
      email: None,
    });
  }
  Ok(match session::load(server)? {
    Some(session) => Credential {
      token: Some(session.token),
      source: CredentialSource::EncryptedSession,
      email: session.email,
    },
    None => Credential {
      token: None,
      source: CredentialSource::None,
      email: None,
    },
  })
}
pub fn save_credential(
  server: &ResolvedServer,
  token: &str,
  email: Option<&str>,
) -> Result<()> {
  session::save(server, token, email)
}
pub fn remove_credential(server: &ResolvedServer) -> Result<bool> {
  session::remove(server)
}
pub async fn login(
  server: &ResolvedServer,
  save: bool,
) -> Result<ApiClient> {
  if !io::stdin().is_terminal() {
    bail!("interactive login requires a terminal; set DOPBASE_TOKEN for automation");
  }
  let (email, password) = prompt_login(server).await?;
  let client = ApiClient::new(server, None)?;
  let request = client.request(
    Method::POST,
    "/api/v1/auth/login",
    Some(json!({"email":email,"password":password,"sessionKind":"cli"})),
  );
  let data = tokio::select! {
    result = request => result?,
    signal = tokio::signal::ctrl_c() => {
      signal?;
      return Err(CliCancelled::Login.into());
    }
  };
  let token = data
    .get("token")
    .and_then(Value::as_str)
    .context("login response did not contain a token")?
    .to_owned();
  let email = data
    .get("email")
    .and_then(Value::as_str)
    .context("login response did not contain an email")?;
  if save {
    save_credential(server, &token, Some(email))?;
  }
  ApiClient::new(server, Some(token))
}

async fn prompt_login(server: &ResolvedServer) -> Result<(String, String)> {
  let server_url = server.url.clone();
  let mut prompt = tokio::task::spawn_blocking(move || prompt_credentials(&server_url));
  tokio::select! {
    result = &mut prompt => result.context("login prompt task failed")?,
    signal = tokio::signal::ctrl_c() => {
      signal?;
      let _ = tokio::time::timeout(Duration::from_millis(250), &mut prompt).await;
      Err(CliCancelled::Login.into())
    }
  }
}

fn prompt_credentials(server_url: &str) -> Result<(String, String)> {
  eprintln!("Dopbase login\nServer: {server_url}\n");
  eprint!("email: ");
  io::stderr().flush()?;
  let mut email = String::new();
  io::stdin()
    .read_line(&mut email)
    .map_err(map_prompt_error)?;
  let email = normalize_login_email(&email)?;
  let password = rpassword::prompt_password("password: ").map_err(map_prompt_error)?;
  Ok((email, password))
}

fn map_prompt_error(error: io::Error) -> anyhow::Error {
  if error.kind() == io::ErrorKind::Interrupted {
    CliCancelled::Login.into()
  } else {
    error.into()
  }
}

pub fn normalize_login_email(value: &str) -> Result<String> {
  crate::modules::common::validate_email(value)
    .map_err(|_| anyhow::anyhow!("Enter a valid email address."))
}
enum HumanClient {
  Existing(ApiClient),
  NewlyAuthenticated(ApiClient),
}

async fn acquire_human_client(server: &ResolvedServer) -> Result<HumanClient> {
  let credential = credential(server)?;
  if let Some(token) = credential.token {
    let client = ApiClient::new(server, Some(token))?;
    if client
      .request(Method::GET, "/api/v1/auth/session", None)
      .await
      .is_ok()
    {
      return Ok(HumanClient::Existing(client));
    }
    if credential.source == CredentialSource::Environment || !io::stdin().is_terminal() {
      bail!("the configured Dopbase credential is invalid or expired");
    }
  }
  login(server, true)
    .await
    .map(HumanClient::NewlyAuthenticated)
}

pub async fn human_client(server: &ResolvedServer) -> Result<ApiClient> {
  Ok(match acquire_human_client(server).await? {
    HumanClient::Existing(client) | HumanClient::NewlyAuthenticated(client) => client,
  })
}

pub async fn password_confirmed_human_client(server: &ResolvedServer) -> Result<ApiClient> {
  if !io::stdin().is_terminal() {
    bail!("interactive password confirmation is required for plaintext secret access");
  }
  match acquire_human_client(server).await? {
    HumanClient::NewlyAuthenticated(client) => Ok(client),
    HumanClient::Existing(client) => {
      let password = prompt_password_confirmation().await?;
      let request = client.request(
        Method::POST,
        "/api/v1/auth/reauthenticate",
        Some(json!({"password":password})),
      );
      tokio::select! {
        result = request => { result?; }
        signal = tokio::signal::ctrl_c() => {
          signal?;
          return Err(CliCancelled::PasswordConfirmation.into());
        }
      }
      Ok(client)
    }
  }
}

async fn prompt_password_confirmation() -> Result<String> {
  eprintln!("Password confirmation required.");
  let mut prompt = tokio::task::spawn_blocking(|| {
    rpassword::prompt_password("password: ").map_err(|error| {
      if error.kind() == io::ErrorKind::Interrupted {
        CliCancelled::PasswordConfirmation.into()
      } else {
        error.into()
      }
    })
  });
  tokio::select! {
    result = &mut prompt => result.context("password confirmation prompt task failed")?,
    signal = tokio::signal::ctrl_c() => {
      signal?;
      let _ = tokio::time::timeout(Duration::from_millis(250), &mut prompt).await;
      Err(CliCancelled::PasswordConfirmation.into())
    }
  }
}
pub async fn any_authenticated_client(server: &ResolvedServer) -> Result<ApiClient> {
  let credential = credential(server)?;
  if let Some(token) = credential.token {
    return ApiClient::new(server, Some(token));
  }
  if credential.source == CredentialSource::Environment || !io::stdin().is_terminal() {
    bail!("Dopbase authentication is required");
  }
  login(server, true).await
}
pub fn encode_query(value: &str) -> String {
  url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
