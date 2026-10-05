use crate::models::SecretInput;
use crate::utils::env_layout::valid_key;
pub use crate::utils::env_layout::validate as validate_layout;
use anyhow::{Context, Result, bail};
use std::{
  collections::{HashMap, HashSet},
  fs,
  path::Path,
};
use zeroize::{Zeroize, Zeroizing};
pub fn parse_file(path: &Path) -> Result<Vec<SecretInput>> {
  let text =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  parse(&text)
}
pub fn parse(text: &str) -> Result<Vec<SecretInput>> {
  let mut entries = Vec::new();
  let mut seen = HashSet::new();
  let result: Result<()> = (|| {
    for (line_number, raw) in text.lines().enumerate() {
      let line = raw.trim();
      if line.is_empty() || line.starts_with('#') {
        continue;
      }
      let line = line.strip_prefix("export ").unwrap_or(line);
      let Some((key, raw_value)) = line.split_once('=') else {
        bail!("invalid .env entry on line {}", line_number + 1)
      };
      let key = key.trim();
      if key.is_empty() {
        bail!("secret keys may not be empty on line {}", line_number + 1);
      }
      if !valid_key(key) {
        bail!("invalid secret key on line {}", line_number + 1);
      }
      if !seen.insert(key.to_owned()) {
        bail!("duplicate key {key} on line {}", line_number + 1)
      };
      let value = parse_value(raw_value.trim())
        .with_context(|| format!("invalid value on line {}", line_number + 1))?;
      entries.push(SecretInput {
        key: key.into(),
        value,
      });
    }
    Ok(())
  })();
  if result.is_err() {
    for entry in &mut entries {
      entry.value.zeroize();
    }
  }
  result?;
  Ok(entries)
}
fn parse_value(value: &str) -> Result<String> {
  if let Some(value) = value.strip_prefix('"') {
    let Some(inner) = value.strip_suffix('"') else {
      bail!("unterminated double quote")
    };
    let mut output = Zeroizing::new(String::new());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
      if ch == '\\' {
        match chars.next() {
          Some('n') => output.push('\n'),
          Some('r') => output.push('\r'),
          Some('t') => output.push('\t'),
          Some('"') => output.push('"'),
          Some('\\') => output.push('\\'),
          Some(other) => {
            output.push('\\');
            output.push(other)
          }
          None => bail!("unfinished escape"),
        }
      } else if ch == '"' {
        bail!("unexpected double quote");
      } else {
        output.push(ch)
      }
    }
    return Ok(std::mem::take(&mut *output));
  }
  if let Some(value) = value.strip_prefix('\'') {
    let Some(inner) = value.strip_suffix('\'') else {
      bail!("unterminated single quote")
    };
    if inner.contains('\'') {
      bail!("unexpected single quote");
    }
    return Ok(inner.into());
  }
  Ok(value.split(" #").next().unwrap_or(value).trim_end().into())
}
pub fn render(entries: &[SecretInput]) -> String {
  let mut output = String::new();
  for entry in entries {
    output.push_str(&entry.key);
    output.push('=');
    output.push_str(&quote(&entry.value));
    output.push('\n');
  }
  output
}
fn quote(value: &str) -> String {
  if value
    .chars()
    .all(|ch| ch.is_ascii_alphanumeric() || "_./:@+-".contains(ch))
  {
    return value.into();
  }
  format!(
    "\"{}\"",
    value
      .replace('\\', "\\\\")
      .replace('"', "\\\"")
      .replace('\n', "\\n")
      .replace('\r', "\\r")
      .replace('\t', "\\t")
  )
}

/// Parses entries and extracts comments and empty slots without retaining values in layout.
pub fn parse_document(text: &str) -> Result<(Vec<SecretInput>, String)> {
  let entries = parse(text)?;
  let mut output = String::new();
  for raw in text.lines() {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
      output.push_str(raw);
    } else {
      let (prefix, value) = line.split_once('=').context("invalid layout entry")?;
      let value = value.trim();
      if !value.starts_with(['\'', '"'])
        && let Some((_, comment)) = value.split_once(" #")
      {
        output.push('#');
        output.push_str(comment);
        output.push('\n');
      }
      output.push_str(prefix.trim_end());
      output.push('=');
    }
    output.push('\n');
  }
  Ok((entries, output))
}

/// Restores live values into layout slots and appends keys without a slot.
pub fn render_layout(
  layout: Option<&str>,
  entries: &[SecretInput],
) -> Result<String> {
  if let Some(layout) = layout {
    validate_layout(layout)?;
  }
  let values: HashMap<_, _> = entries
    .iter()
    .map(|entry| (entry.key.as_str(), entry.value.as_str()))
    .collect();
  let mut placed = HashSet::new();
  let mut output = String::new();
  for raw in layout.unwrap_or_default().lines() {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
      output.push_str(raw);
      output.push('\n');
      continue;
    }
    let export = line.starts_with("export ");
    let key = line
      .strip_prefix("export ")
      .unwrap_or(line)
      .split_once('=')
      .unwrap()
      .0
      .trim();
    if let Some(value) = values.get(key) {
      if export {
        output.push_str("export ");
      }
      output.push_str(key);
      output.push('=');
      output.push_str(&quote(value));
      output.push('\n');
      placed.insert(key);
    }
  }
  for entry in entries {
    if !placed.contains(entry.key.as_str()) {
      output.push_str(&entry.key);
      output.push('=');
      output.push_str(&quote(&entry.value));
      output.push('\n');
    }
  }
  Ok(output)
}
