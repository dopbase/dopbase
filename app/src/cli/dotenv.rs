use crate::models::SecretInput;
use anyhow::{Context, Result, bail};
use std::{collections::HashSet, fs, path::Path};
pub fn parse_file(path: &Path) -> Result<Vec<SecretInput>> {
  let text =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  parse(&text)
}
pub fn parse(text: &str) -> Result<Vec<SecretInput>> {
  let mut entries = Vec::new();
  let mut seen = HashSet::new();
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
  Ok(entries)
}
fn parse_value(value: &str) -> Result<String> {
  if let Some(value) = value.strip_prefix('"') {
    let Some(inner) = value.strip_suffix('"') else {
      bail!("unterminated double quote")
    };
    let mut output = String::new();
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
      } else {
        output.push(ch)
      }
    }
    return Ok(output);
  }
  if let Some(value) = value.strip_prefix('\'') {
    let Some(inner) = value.strip_suffix('\'') else {
      bail!("unterminated single quote")
    };
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
  )
}
