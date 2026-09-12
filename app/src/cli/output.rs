use anstyle::{AnsiColor, Color, Effects, Style};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{io::Write, path::Path};

const HEADING: Style = Style::new().effects(Effects::BOLD);
const SUCCESS: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
const WARNING: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

pub(super) fn print_json(value: &Value) -> Result<()> {
  anstream::println!("{}", serde_json::to_string_pretty(value)?);
  Ok(())
}

pub(super) fn print_value(
  json_output: bool,
  value: &Value,
) {
  debug_assert!(
    json_output,
    "human output must use a command-specific renderer"
  );
  print_json(value).expect("serializing a JSON value cannot fail");
}

pub(super) fn print_text(value: &str) {
  anstream::println!("{value}");
}

pub(super) fn print_raw(value: &str) -> Result<()> {
  let mut stdout = std::io::stdout().lock();
  stdout.write_all(value.as_bytes())?;
  stdout.flush()?;
  Ok(())
}

pub(super) fn print_success(message: &str) {
  anstream::println!("{SUCCESS}{message}{SUCCESS:#}");
}

pub(super) fn print_warning(message: &str) {
  anstream::eprintln!("{WARNING}Warning:{WARNING:#} {message}");
}

pub(super) fn print_progress(
  step: usize,
  total: usize,
  message: &str,
) {
  anstream::eprintln!("{HEADING}[{step}/{total}]{HEADING:#} {message}");
}

pub(super) fn print_fields(fields: &[(&str, String)]) {
  print_text(&render_fields(fields));
}

pub(super) fn print_table(
  headers: &[&str],
  rows: &[Vec<String>],
  empty_message: &str,
  summary: &str,
) {
  if rows.is_empty() {
    print_text(empty_message);
    return;
  }

  let widths = column_widths(headers, rows);
  let header = render_row(
    &headers
      .iter()
      .map(|value| (*value).to_owned())
      .collect::<Vec<_>>(),
    &widths,
  );
  anstream::println!("{HEADING}{header}{HEADING:#}");
  for row in rows {
    anstream::println!("{}", render_row(row, &widths));
  }
  if !summary.is_empty() {
    anstream::println!("\n{summary}");
  }
}

pub fn render_fields(fields: &[(&str, String)]) -> String {
  let width = fields
    .iter()
    .map(|(label, _)| label.chars().count())
    .max()
    .unwrap_or(0);
  fields
    .iter()
    .map(|(label, value)| format!("{label:<width$}  {value}"))
    .collect::<Vec<_>>()
    .join("\n")
}

pub fn render_table(
  headers: &[&str],
  rows: &[Vec<String>],
) -> String {
  let widths = column_widths(headers, rows);
  std::iter::once(render_row(
    &headers
      .iter()
      .map(|value| (*value).to_owned())
      .collect::<Vec<_>>(),
    &widths,
  ))
  .chain(rows.iter().map(|row| render_row(row, &widths)))
  .collect::<Vec<_>>()
  .join("\n")
}

fn column_widths(
  headers: &[&str],
  rows: &[Vec<String>],
) -> Vec<usize> {
  headers
    .iter()
    .enumerate()
    .map(|(index, header)| {
      rows
        .iter()
        .filter_map(|row| row.get(index))
        .map(|value| value.chars().count())
        .fold(header.chars().count(), usize::max)
    })
    .collect()
}

fn render_row(
  row: &[String],
  widths: &[usize],
) -> String {
  row
    .iter()
    .enumerate()
    .map(|(index, value)| {
      if index + 1 == row.len() {
        value.clone()
      } else {
        let padding = widths.get(index).copied().unwrap_or_default() - value.chars().count();
        format!("{value}{}", " ".repeat(padding + 3))
      }
    })
    .collect()
}

pub(super) fn string(
  value: &Value,
  field: &str,
) -> String {
  value
    .get(field)
    .and_then(Value::as_str)
    .unwrap_or("-")
    .to_owned()
}

pub(super) fn timestamp(
  value: &Value,
  field: &str,
) -> String {
  match value.get(field).and_then(Value::as_str) {
    Some(raw) => DateTime::parse_from_rfc3339(raw)
      .map(|time| {
        time
          .with_timezone(&Utc)
          .format("%Y-%m-%d %H:%M UTC")
          .to_string()
      })
      .unwrap_or_else(|_| raw.to_owned()),
    None => "never".into(),
  }
}

pub(super) fn array(value: &Value) -> &[Value] {
  value.as_array().map(Vec::as_slice).unwrap_or_default()
}

pub(super) fn number(
  value: &Value,
  field: &str,
) -> u64 {
  value.get(field).and_then(Value::as_u64).unwrap_or_default()
}

pub(super) fn write_private(
  path: &Path,
  contents: &[u8],
  force: bool,
) -> Result<()> {
  crate::utils::private_file::write(path, contents, force)
}
