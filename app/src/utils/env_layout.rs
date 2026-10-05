use anyhow::{Context, Result, bail};
use std::collections::HashSet;

pub(crate) fn valid_key(key: &str) -> bool {
  let mut bytes = key.bytes();
  key.len() <= 128
    && bytes
      .next()
      .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == b'_')
    && bytes.all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
}

/// Checks the metadata grammar without accepting values in layout slots.
pub fn validate(layout: &str) -> Result<()> {
  let mut seen = HashSet::new();
  for raw in layout.lines() {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    let line = line.strip_prefix("export ").unwrap_or(line).trim();
    let (key, value) = line.split_once('=').context("invalid layout slot")?;
    if !valid_key(key.trim()) || !value.trim().is_empty() || !seen.insert(key.trim()) {
      bail!("invalid layout slot");
    }
  }
  Ok(())
}
