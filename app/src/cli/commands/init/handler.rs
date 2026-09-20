use crate::{
  cli::{
    client::{self, ApiClient, CliCancelled},
    environment_target,
    local_config::ResolvedServer,
    output, prompt, secret_format,
  },
  constants::api,
  models::SecretInput,
};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
  collections::HashSet,
  fs,
  io::{self, ErrorKind, IsTerminal},
  path::Path,
};

pub(crate) async fn execute(
  server: &ResolvedServer,
  args: super::InitArgs,
  json_output: bool,
) -> Result<i32> {
  match (args.target, args.from) {
    (Some(target), Some(from)) => explicit(server, target, from, args.format, json_output).await,
    (None, None) => interactive(server, args.format, json_output).await,
    _ => unreachable!("clap requires init target and --from together"),
  }
}

async fn explicit(
  server: &ResolvedServer,
  target: environment_target::EnvironmentTarget,
  from: std::path::PathBuf,
  format: Option<secret_format::SecretFormat>,
  json_output: bool,
) -> Result<i32> {
  let (project, environment) = target.into_parts();
  let format = secret_format::SecretFormat::for_input(&from, format)?;
  let api = client::human_client(server).await?;
  let entries = secret_format::read(&from, Some(format))?;
  let data = initialize(&api, &project, &environment, &entries).await?;
  if json_output {
    output::print_json(&data)?;
  } else {
    let created_project = data.get("project").unwrap_or(&Value::Null);
    output::print_success(&format!("Initialized {project}/{environment}."));
    output::print_fields(&[
      ("Project ID:", output::string(created_project, "id")),
      ("Environment ID:", output::string(&data, "environmentId")),
      (
        "Secrets:",
        data
          .get("secretCount")
          .and_then(Value::as_u64)
          .unwrap_or_default()
          .to_string(),
      ),
    ]);
  }
  Ok(0)
}

async fn interactive(
  server: &ResolvedServer,
  format: Option<secret_format::SecretFormat>,
  json_output: bool,
) -> Result<i32> {
  if json_output {
    bail!(
      "interactive init does not support --json. Pass PROJECT_NAME/ENVIRONMENT_NAME and --from for machine-readable output"
    );
  }
  if format.is_some() {
    unreachable!("clap requires --from when --format is present");
  }
  if !io::stdin().is_terminal() {
    bail!(
      "interactive init requires a terminal. Pass PROJECT_NAME/ENVIRONMENT_NAME and --from for non-interactive use"
    );
  }

  ApiClient::new(server, None)?.health().await?;
  let api_client = client::human_client(server).await?;
  let path = Path::new(".env");
  let source = fs::read(path).context("could not read .env in the current directory")?;
  let text = std::str::from_utf8(&source).context(".env must contain valid UTF-8 text")?;
  let entries = secret_format::parse(text, secret_format::SecretFormat::Dotenv)?;
  let (sensitive, non_sensitive) = classify_entries(&entries);

  output::print_text(".env found");
  output::print_text("");
  output::print_text(&render_detection_summary(
    entries.len(),
    sensitive,
    non_sensitive,
  ));
  output::print_text("");
  if !prompt::choice("Import into Dopbase?", false, CliCancelled::Confirmation)? {
    output::print_text("Import cancelled.");
    return Ok(0);
  }

  let projects = api_client
    .request(Method::GET, api::projects::COLLECTION, None)
    .await?;
  let mut existing = project_names(&projects);
  let (project, environment, data) = loop {
    let raw = prompt::text(
      "Project/environment (for example payment-service/local):",
      CliCancelled::Confirmation,
    )?;
    let (project, environment) = match validate_interactive_target(raw.trim(), &existing) {
      Ok(target) => target,
      Err(error) => {
        output::print_warning(&error);
        continue;
      }
    };
    match initialize(&api_client, &project, &environment, &entries).await {
      Ok(data) => break (project, environment, data),
      Err(error) if client::is_conflict_error(&error) => {
        existing.insert(project.clone());
        output::print_warning(&format!(
          "Project {project} already exists. Enter a different project name."
        ));
      }
      Err(error) => return Err(error),
    }
  };

  let imported = data
    .get("secretCount")
    .and_then(Value::as_u64)
    .unwrap_or(entries.len() as u64);
  output::print_success(&format!(
    "✓ Imported {imported} {} into {project}/{environment}.",
    if imported == 1 {
      "variable"
    } else {
      "variables"
    }
  ));
  output::print_success("✓ Secrets encrypted.");

  if prompt::choice("Delete .env now?", true, CliCancelled::Confirmation)? {
    match remove_env_if_unchanged(path, &source)? {
      EnvCleanup::Deleted => output::print_success("✓ .env deleted."),
      EnvCleanup::Missing => output::print_warning(".env was already removed."),
      EnvCleanup::Changed => output::print_warning(
        "Import succeeded, but .env changed after it was read and was not deleted.",
      ),
    }
  } else {
    output::print_warning(".env was kept. Do not commit it.");
  }

  output::print_text(&format!(
    "\nRun your application with:\n\n  dopbase run {project}/{environment} -- <your-command>"
  ));
  Ok(0)
}

async fn initialize(
  api_client: &ApiClient,
  project: &str,
  environment: &str,
  entries: &[SecretInput],
) -> Result<Value> {
  api_client
    .request(
      Method::POST,
      api::projects::INIT,
      Some(json!({
        "projectName": project,
        "environmentName": environment,
        "entries": entries,
      })),
    )
    .await
}

fn project_names(projects: &Value) -> HashSet<String> {
  projects
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(|project| project.get("name").and_then(Value::as_str))
    .map(str::to_owned)
    .collect()
}

fn sentence(message: &str) -> String {
  let mut message = message.to_owned();
  if let Some(first) = message.get_mut(0..1) {
    first.make_ascii_uppercase();
  }
  if !message.ends_with('.') {
    message.push('.');
  }
  message
}

#[doc(hidden)]
pub fn validate_interactive_target(
  raw: &str,
  existing: &HashSet<String>,
) -> std::result::Result<(String, String), String> {
  let target = environment_target::parse_init(raw).map_err(|error| sentence(&error))?;
  let (project, environment) = target.into_parts();
  if existing.contains(&project) {
    return Err(format!(
      "Project {project} already exists. Enter a different project name."
    ));
  }
  Ok((project, environment))
}

#[doc(hidden)]
pub fn classify_entries(entries: &[SecretInput]) -> (usize, usize) {
  let sensitive = entries
    .iter()
    .filter(|entry| is_sensitive_key(&entry.key))
    .count();
  (sensitive, entries.len() - sensitive)
}

fn is_sensitive_key(key: &str) -> bool {
  const WORDS: &[&str] = &[
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PASSWD",
    "PWD",
    "CREDENTIAL",
    "CREDENTIALS",
  ];
  const COMPOUNDS: &[&str] = &[
    "API_KEY",
    "ACCESS_KEY",
    "PRIVATE_KEY",
    "CLIENT_SECRET",
    "DATABASE_URL",
    "DB_URL",
    "CONNECTION_STRING",
    "AUTH_KEY",
    "AUTH_TOKEN",
  ];
  let normalized = key.to_ascii_uppercase();
  let words = normalized
    .split(|character: char| !character.is_ascii_alphanumeric())
    .collect::<Vec<_>>();
  WORDS.iter().any(|marker| words.contains(marker))
    || COMPOUNDS.iter().any(|marker| normalized.contains(marker))
}

#[doc(hidden)]
pub fn render_detection_summary(
  total: usize,
  sensitive: usize,
  non_sensitive: usize,
) -> String {
  format!(
    "{total} {} detected\n{sensitive} {} sensitive\n{non_sensitive} {} non-sensitive",
    if total == 1 { "variable" } else { "variables" },
    if sensitive == 1 { "appears" } else { "appear" },
    if non_sensitive == 1 {
      "appears"
    } else {
      "appear"
    },
  )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvCleanup {
  Deleted,
  Missing,
  Changed,
}

#[doc(hidden)]
pub fn remove_env_if_unchanged(
  path: &Path,
  expected: &[u8],
) -> Result<EnvCleanup> {
  let current = match fs::read(path) {
    Ok(current) => current,
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(EnvCleanup::Missing),
    Err(error) => {
      return Err(error).with_context(|| {
        format!(
          "import succeeded, but {} could not be checked",
          path.display()
        )
      });
    }
  };
  if current != expected {
    return Ok(EnvCleanup::Changed);
  }
  fs::remove_file(path).with_context(|| {
    format!(
      "import succeeded, but {} could not be deleted",
      path.display()
    )
  })?;
  Ok(EnvCleanup::Deleted)
}
