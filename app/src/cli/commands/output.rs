use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub(super) fn print_value(
  json_output: bool,
  value: &Value,
) {
  if json_output {
    println!(
      "{}",
      serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into())
    );
  } else if value.is_null() {
    println!("Done.");
  } else if let Some(value) = value.as_str() {
    println!("{value}");
  } else {
    println!(
      "{}",
      serde_json::to_string_pretty(value).unwrap_or_else(|_| "Done.".into())
    );
  }
}
