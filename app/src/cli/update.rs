use std::time::Duration;

use anstyle::{AnsiColor, Color, Effects, Style};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

const RELEASE_API_URL: &str = "https://api.github.com/repos/dopbase/dopbase/releases/latest";
const RELEASES_PAGE_URL: &str = "https://github.com/dopbase/dopbase/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const HEADING: Style = Style::new()
  .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
  .effects(Effects::BOLD);
const VERSION: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
const COMMAND: Style = Style::new()
  .fg_color(Some(Color::Ansi(AnsiColor::Green)))
  .effects(Effects::BOLD);

#[derive(Debug, Serialize)]
pub struct UpdateStatus {
  pub current_version: &'static str,
  pub latest_version: String,
  pub update_available: bool,
  pub release_url: String,
}

pub async fn run(json_output: bool) -> Result<i32> {
  let current = env!("CARGO_PKG_VERSION");
  let status = check(current).await?;
  if json_output {
    println!("{}", serde_json::to_string_pretty(&status)?);
  } else if status.update_available {
    anstream::println!("{}", update_message(&status));
  } else {
    println!(
      "dopbase {current} is up to date (latest release {}).",
      status.latest_version
    );
  }
  Ok(0)
}

#[doc(hidden)]
pub fn update_message(status: &UpdateStatus) -> String {
  format!(
    "{HEADING}A new Dopbase release is available{HEADING:#}\n\n\
     Current version  {VERSION}{}{VERSION:#}\n\
     Latest version   {VERSION}{}{VERSION:#}\n\
     Release notes    {}\n\n\
     {HEADING}Stop every running Dopbase server before updating.{HEADING:#}\n\
     Then run:\n\n  {COMMAND}curl -fsSL https://dopbase.com/install.sh | sh{COMMAND:#}",
    status.current_version, status.latest_version, status.release_url
  )
}

async fn check(current: &'static str) -> Result<UpdateStatus> {
  let current_version = parse_version(current).context("invalid CARGO_PKG_VERSION")?;
  let client = reqwest::Client::builder()
    .user_agent(format!("dopbase/{current}"))
    .timeout(REQUEST_TIMEOUT)
    .build()
    .context("failed to build an HTTP client")?;
  let response = client
    .get(RELEASE_API_URL)
    .header("Accept", "application/vnd.github+json")
    .header("X-GitHub-Api-Version", "2022-11-28")
    .send()
    .await
    .context("failed to query the latest Dopbase release")?;
  if response.status() == reqwest::StatusCode::NOT_FOUND {
    bail!("no published Dopbase release found yet ({RELEASES_PAGE_URL})");
  }
  let payload: Value = response
    .error_for_status()
    .context("the latest-release request was rejected")?
    .json()
    .await
    .context("failed to decode the latest-release response")?;
  let release = parse_release(&payload)?;
  let update_available = release
    .version
    .map(|latest| is_newer(latest, current_version))
    .unwrap_or(false);
  let latest_version = release.tag;
  Ok(UpdateStatus {
    current_version: current,
    latest_version,
    update_available,
    release_url: release.url,
  })
}

pub struct ReleaseInfo {
  pub tag: String,
  pub version: Option<(u64, u64, u64)>,
  pub url: String,
}

pub fn parse_release(payload: &Value) -> Result<ReleaseInfo> {
  let tag = payload
    .get("tag_name")
    .and_then(Value::as_str)
    .context("the release payload has no tag_name")?
    .to_string();
  let url = payload
    .get("html_url")
    .and_then(Value::as_str)
    .unwrap_or(RELEASES_PAGE_URL)
    .to_string();
  if let Some(version) = parse_version(&tag) {
    Ok(ReleaseInfo {
      tag,
      version: Some(version),
      url,
    })
  } else {
    // Pre-release or malformed tags cannot be ordered against the current
    // version — fail closed instead of claiming "up to date".
    bail!("the latest release tag {tag:?} is not a MAJOR.MINOR.PATCH version");
  }
}

pub fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
  let value = value.trim().strip_prefix('v').unwrap_or(value.trim());
  let mut components = value.split('.');
  let major = components.next()?;
  let minor = components.next()?;
  let patch = components.next()?;
  if components.next().is_some()
    || major.is_empty()
    || minor.is_empty()
    || patch.is_empty()
    || major.bytes().any(|byte| !byte.is_ascii_digit())
    || minor.bytes().any(|byte| !byte.is_ascii_digit())
    || patch.bytes().any(|byte| !byte.is_ascii_digit())
  {
    return None;
  }
  Some((
    major.parse().ok()?,
    minor.parse().ok()?,
    patch.parse().ok()?,
  ))
}

pub fn is_newer(
  latest: (u64, u64, u64),
  current: (u64, u64, u64),
) -> bool {
  latest > current
}
