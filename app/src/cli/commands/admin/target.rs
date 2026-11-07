//! Which instance an admin command acts on.
//!
//! Admin commands open the local data directory directly and never talk to
//! the server the CLI is connected to. When that connection points at some
//! other server, running one would quietly change the wrong instance, so
//! they refuse instead.

use crate::cli::local_config::{self, ServerSource};
use crate::config::ServerConfig;
use crate::server::InstanceLock;
use anyhow::{Result, bail};
use serde_json::Value;
use std::{net::SocketAddr, time::Duration};
use url::{Host, Url};

/// How long to wait for a server at the connected address to answer.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// Refuse `dopbase admin <command>` unless the CLI's saved connection
/// (client config or `DOPBASE_URL`) points at the instance in `config`.
/// With no saved connection there is nothing to contradict, so it passes.
///
/// An address can look like this instance and still be another one: a
/// container published on this instance's port answers on the same
/// loopback address. So when this instance is stopped and a Dopbase server
/// answers at the connected address anyway, that server is someone else.
pub(super) async fn ensure_connected_here(
  command: &str,
  config: &ServerConfig,
) -> Result<()> {
  let connection = local_config::resolve(None, Some(&config.data_dir))?;
  if matches!(connection.source, ServerSource::Default) {
    return Ok(());
  }
  let url = &connection.url;
  if !connection_is_local(url, &config.public_url, &config.bind_address) {
    bail!("please connect to local instance to perform {command} action");
  }
  if !InstanceLock::is_held(&config.database_url)? && answers_as_dopbase(url).await {
    bail!(
      "another Dopbase server is answering at {url}, please run {command} action on that server"
    );
  }
  Ok(())
}

/// Whether a Dopbase server answers its health check at `url`.
async fn answers_as_dopbase(url: &str) -> bool {
  let Ok(client) = reqwest::Client::builder()
    .timeout(PROBE_TIMEOUT)
    .redirect(reqwest::redirect::Policy::none())
    .build()
  else {
    return false;
  };
  let health = format!(
    "{}{}",
    url.trim_end_matches('/'),
    crate::constants::api::health::ROOT
  );
  let Ok(response) = client.get(health).send().await else {
    return false;
  };
  let Ok(body) = response.json::<Value>().await else {
    return false;
  };
  body
    .get("data")
    .unwrap_or(&body)
    .get("product")
    .and_then(Value::as_str)
    == Some("dopbase")
}

/// Whether `connected` reaches the instance with this public URL and bind
/// address: either it is the public URL, or a loopback address on the port
/// the server listens on.
pub fn connection_is_local(
  connected: &str,
  public_url: &str,
  bind_address: &str,
) -> bool {
  let Ok(connected) = Url::parse(connected) else {
    return false;
  };
  if let Ok(public) = Url::parse(public_url)
    && same_endpoint(&connected, &public)
  {
    return true;
  }
  let Ok(bind) = bind_address.parse::<SocketAddr>() else {
    return false;
  };
  connected.scheme() == "http"
    && is_loopback(&connected)
    && connected.port_or_known_default() == Some(bind.port())
}

fn same_endpoint(
  left: &Url,
  right: &Url,
) -> bool {
  let hosts_match = match (is_loopback(left), is_loopback(right)) {
    (true, true) => true,
    (false, false) => left.host_str() == right.host_str(),
    _ => false,
  };
  hosts_match
    && left.scheme() == right.scheme()
    && left.port_or_known_default() == right.port_or_known_default()
    && left.path().trim_end_matches('/') == right.path().trim_end_matches('/')
}

fn is_loopback(url: &Url) -> bool {
  match url.host() {
    Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
    Some(Host::Ipv4(host)) => host.is_loopback(),
    Some(Host::Ipv6(host)) => host.is_loopback(),
    None => false,
  }
}
