use app::{
  cli::dotenv::{parse, parse_document, render, render_layout, validate_layout},
  models::SecretInput,
};
use proptest::{prelude::*, string::string_regex};
use std::collections::HashSet;

/// Keys accepted by the dotenv grammar: a leading letter or underscore,
/// followed by letters, digits, or underscores, within the 128 byte limit.
fn valid_key() -> impl Strategy<Value = String> {
  string_regex("[A-Za-z_][A-Za-z0-9_]{0,15}")
    .expect("the secret key pattern is a valid regular expression")
}

/// Arbitrary entries with unique, valid keys and completely arbitrary values.
fn secret_entries() -> impl Strategy<Value = Vec<SecretInput>> {
  prop::collection::vec((valid_key(), any::<String>()), 0..16).prop_map(dedup_entries)
}

fn dedup_entries(rows: Vec<(String, String)>) -> Vec<SecretInput> {
  let mut seen = HashSet::new();
  rows
    .into_iter()
    .filter(|(key, _)| seen.insert(key.clone()))
    .map(|(key, value)| SecretInput { key, value })
    .collect()
}

fn key_values(entries: &[SecretInput]) -> Vec<(&str, &str)> {
  entries
    .iter()
    .map(|entry| (entry.key.as_str(), entry.value.as_str()))
    .collect()
}

/// The same entries with every value dropped, which is what a saved layout
/// must look like: `KEY=` slots preserving order, comments, and blank lines.
fn slot_only(entries: &[SecretInput]) -> String {
  entries
    .iter()
    .map(|entry| format!("{}=\n", entry.key))
    .collect()
}

proptest! {
  #[test]
  fn rendered_entries_round_trip_through_the_parser(entries in secret_entries()) {
    let parsed = parse(&render(&entries)).expect("rendered entries always parse");
    prop_assert_eq!(key_values(&parsed), key_values(&entries));
  }

  #[test]
  fn parsed_documents_keep_a_value_free_layout(entries in secret_entries()) {
    let (parsed, layout) =
      parse_document(&render(&entries)).expect("rendered entries always parse");
    prop_assert_eq!(key_values(&parsed), key_values(&entries));
    // The layout never retains values: every non-comment line is an empty slot.
    prop_assert_eq!(&layout, &slot_only(&entries));
    validate_layout(&layout).expect("a generated layout always validates");
  }

  #[test]
  fn layouts_restore_exactly_the_rendered_entries(entries in secret_entries()) {
    let rendered = render(&entries);
    let (_, layout) = parse_document(&rendered).expect("rendered entries always parse");
    let restored =
      render_layout(Some(&layout), &entries).expect("a generated layout always renders");
    prop_assert_eq!(restored, rendered);
  }

  #[test]
  fn arbitrary_value_bytes_never_escape_a_single_entry(value in any::<String>()) {
    let entries = vec![SecretInput {
      key: "KEY".into(),
      value,
    }];
    let parsed = parse(&render(&entries)).expect("rendered entries always parse");
    prop_assert_eq!(key_values(&parsed), key_values(&entries));
  }
}
