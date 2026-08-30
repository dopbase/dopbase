use super::local_config::ResolvedServer;
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
  env,
  io::{self, IsTerminal, Write},
};

pub struct ApiClient {
  pub base_url: String,
  client: reqwest::Client,
  token: Option<String>,
}
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
      .with_context(|| format!("could not connect to {}", self.base_url))?;
    let status = response.status();
    let value: Value = response
      .json()
      .await
      .context("server returned an invalid JSON response")?;
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
}
pub fn credential(server: &ResolvedServer) -> Result<(Option<String>, &'static str)> {
  if let Ok(token) = env::var("DOPBASE_TOKEN") {
    return Ok((Some(token), "environment"));
  }
  let entry = keyring::Entry::new("dopbase", &server.url)?;
  match entry.get_password() {
    Ok(value) => Ok((Some(value), "credential_store")),
    Err(keyring::Error::NoEntry) => Ok((None, "none")),
    Err(error) => Err(error.into()),
  }
}
pub fn save_credential(
  server_url: &str,
  token: &str,
) -> Result<()> {
  keyring::Entry::new("dopbase", server_url)?.set_password(token)?;
  Ok(())
}
pub fn remove_credential(server_url: &str) -> Result<()> {
  let entry = keyring::Entry::new("dopbase", server_url)?;
  match entry.delete_credential() {
    Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
    Err(error) => Err(error.into()),
  }
}
pub async fn login(
  server: &ResolvedServer,
  save: bool,
) -> Result<ApiClient> {
  if !io::stdin().is_terminal() {
    bail!("interactive login requires a terminal; set DOPBASE_TOKEN for automation");
  }
  print!("Email: ");
  io::stdout().flush()?;
  let mut email = String::new();
  io::stdin().read_line(&mut email)?;
  let password = rpassword::prompt_password("Password: ")?;
  let client = ApiClient::new(server, None)?;
  let data = client
    .request(
      Method::POST,
      "/api/v1/auth/login",
      Some(json!({"email":email.trim(),"password":password,"sessionKind":"cli"})),
    )
    .await?;
  let token = data
    .get("token")
    .and_then(Value::as_str)
    .context("login response did not contain a token")?
    .to_owned();
  if save {
    save_credential(&server.url, &token)?;
  }
  ApiClient::new(server, Some(token))
}
pub async fn human_client(server: &ResolvedServer) -> Result<ApiClient> {
  let (token, source) = credential(server)?;
  if let Some(token) = token {
    let client = ApiClient::new(server, Some(token))?;
    if client
      .request(Method::GET, "/api/v1/auth/session", None)
      .await
      .is_ok()
    {
      return Ok(client);
    }
    if source == "environment" || !io::stdin().is_terminal() {
      bail!("the configured Dopbase credential is invalid or expired");
    }
  }
  login(server, true).await
}
pub async fn any_authenticated_client(server: &ResolvedServer) -> Result<ApiClient> {
  let (token, source) = credential(server)?;
  if let Some(token) = token {
    return ApiClient::new(server, Some(token));
  }
  if source == "environment" || !io::stdin().is_terminal() {
    bail!("Dopbase authentication is required");
  }
  login(server, true).await
}
pub fn encode_query(value: &str) -> String {
  url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
