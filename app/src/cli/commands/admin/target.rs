//! Which instance an admin command acts on.
//!
//! Admin commands open the local data directory directly and never talk to
//! the server the CLI is connected to. When that connection points at some
//! other server, running one would quietly change the wrong instance, so
//! they refuse instead.

use crate::cli::local_config::{self, ServerSource};
use crate::config::ServerConfig;
use anyhow::Result;
use std::net::SocketAddr;
use url::{Host, Url};

/// Whether the CLI's saved connection (client config or `DOPBASE_URL`)
/// points at the instance in `config`. With no saved connection there is
/// nothing to contradict, so this is true. Each command refuses with its own
/// message when this is false.
pub(super) fn connected_here(config: &ServerConfig) -> Result<bool> {
  let connection = local_config::resolve(None, Some(&config.data_dir))?;
  Ok(
    matches!(connection.source, ServerSource::Default)
      || connection_is_local(&connection.url, &config.public_url, &config.bind_address),
  )
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
