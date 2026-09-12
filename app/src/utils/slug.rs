use regex::Regex;
use std::sync::OnceLock;

pub fn is_valid(value: &str) -> bool {
  static SLUG: OnceLock<Regex> = OnceLock::new();
  let valid = SLUG.get_or_init(|| Regex::new(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$").unwrap());
  value.len() <= 63 && valid.is_match(value)
}
