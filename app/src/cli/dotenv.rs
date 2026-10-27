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
  let lines: Vec<&str> = text.lines().collect();
  let mut entries = Vec::new();
  let mut seen = HashSet::new();
  let mut i = 0;
  let result: Result<()> = (|| {
    while i < lines.len() {
      let line_number = i + 1;
      let raw = lines[i];
      i += 1;
      let line = raw.trim();
      if line.is_empty() || line.starts_with('#') {
        continue;
      }
      let line = line.strip_prefix("export ").unwrap_or(line);
      let Some((key, raw_value)) = line.split_once('=') else {
        bail!("invalid .env entry on line {}", line_number)
      };
      let key = key.trim();
      if key.is_empty() {
        bail!("secret keys may not be empty on line {}", line_number);
      }
      if !valid_key(key) {
        bail!("invalid secret key on line {}", line_number);
      }
      if !seen.insert(key.to_owned()) {
        bail!("duplicate key {key} on line {}", line_number)
      }
      let value = if raw_value.trim().starts_with("\"\"\"") {
        let (value, next_i) = parse_triple_quoted(raw_value.trim(), &lines, i)
          .with_context(|| format!("invalid value on line {}", line_number))?;
        i = next_i;
        value
      } else {
        parse_value(raw_value.trim())
          .with_context(|| format!("invalid value on line {}", line_number))?
      };
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

/// `KEY="""` … `"""` (PEM and other multiline secrets).
fn parse_triple_quoted(
  opening: &str,
  following: &[&str],
  next_line_idx: usize,
) -> Result<(String, usize)> {
  let body = opening
    .strip_prefix("\"\"\"")
    .context("expected triple double quote opener")?;

  if body.ends_with("\"\"\"") {
    let inner = body
      .strip_suffix("\"\"\"")
      .context("expected triple double quote closer")?;
    return Ok((inner.to_string(), next_line_idx));
  }

  let mut buf = String::new();
  if !body.is_empty() {
    buf.push_str(body);
  }
  let mut i = next_line_idx;
  while i < following.len() {
    let raw = following[i];
    i += 1;
    let trimmed_end = raw.trim_end();
    if trimmed_end.ends_with("\"\"\"") {
      let part = trimmed_end.strip_suffix("\"\"\"").unwrap_or(trimmed_end);
      if !part.is_empty() {
        if !buf.is_empty() {
          buf.push('\n');
        }
        buf.push_str(part);
      }
      return Ok((buf, i));
    }
    if !buf.is_empty() {
      buf.push('\n');
    }
    buf.push_str(raw);
  }
  bail!("unterminated triple double quote")
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
  if value.contains('\n') && !value.contains('\r') {
    let body = value.trim_end_matches('\n');
    return format!("\"\"\"\n{body}\n\"\"\"");
  }
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
  let lines: Vec<&str> = text.lines().collect();
  let mut output = String::new();
  let mut i = 0;
  while i < lines.len() {
    let raw = lines[i];
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
      output.push_str(raw);
      output.push('\n');
      i += 1;
      continue;
    }
    let had_export = line.starts_with("export ");
    let stripped = line.strip_prefix("export ").unwrap_or(line);
    let Some((prefix, value)) = stripped.split_once('=') else {
      bail!("invalid layout entry");
    };
    let value_trim = value.trim();
    if had_export {
      output.push_str("export ");
    }
    if value_trim.starts_with("\"\"\"") {
      output.push_str(prefix.trim_end());
      output.push('=');
      output.push('\n');
      let (_, next_i) = parse_triple_quoted(value_trim, &lines, i + 1)?;
      i = next_i;
      continue;
    }
    if !value_trim.starts_with(['\'', '"'])
      && let Some((_, comment)) = value_trim.split_once(" #")
    {
      output.push('#');
      output.push_str(comment);
      output.push('\n');
    }
    output.push_str(prefix.trim_end());
    output.push('=');
    output.push('\n');
    i += 1;
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
