use app::utils::env_layout::validate;
use proptest::{prelude::*, string::string_regex};
use std::collections::HashSet;

/// Keys accepted by the layout grammar: a leading letter or underscore,
/// followed by letters, digits, or underscores, within the 128 byte limit.
fn valid_key() -> impl Strategy<Value = String> {
  string_regex("[A-Za-z_][A-Za-z0-9_]{0,15}")
    .expect("the secret key pattern is a valid regular expression")
}

/// One row per candidate slot line: the key, whether it carries an `export`
/// prefix, and whether a comment block precedes it.
fn slot_rows() -> impl Strategy<Value = Vec<(String, bool, bool)>> {
  prop::collection::vec((valid_key(), any::<bool>(), any::<bool>()), 0..12)
}

fn render_slots(rows: Vec<(String, bool, bool)>) -> String {
  let mut seen = HashSet::new();
  let mut output = String::new();
  for (key, exported, commented) in rows {
    if !seen.insert(key.clone()) {
      continue;
    }
    if commented {
      output.push_str("# a note\n\n");
    }
    if exported {
      output.push_str("export ");
    }
    output.push_str(&key);
    output.push_str("=\n");
  }
  output
}

proptest! {
  #[test]
  fn layouts_of_unique_empty_slots_are_accepted(rows in slot_rows()) {
    prop_assert!(validate(&render_slots(rows)).is_ok());
  }

  #[test]
  fn layouts_reject_values_inside_slots(
    key in valid_key(),
    value in string_regex("\\S{1,16}").expect("a non-whitespace pattern")
  ) {
    let layout = format!("{key}={value}\n");
    prop_assert!(validate(&layout).is_err());
  }

  #[test]
  fn layouts_reject_duplicate_slots(
    key in valid_key(),
    first_exported in any::<bool>(),
    second_exported in any::<bool>()
  ) {
    let slot = |exported: bool| format!("{}{key}=\n", if exported { "export " } else { "" });
    let layout = format!("{}{}", slot(first_exported), slot(second_exported));
    prop_assert!(validate(&layout).is_err());
  }

  #[test]
  fn layouts_reject_keys_outside_the_slot_grammar(key in valid_key()) {
    for candidate in [format!("-{key}="), format!("1{key}="), format!("{key} ")] {
      let layout = format!("{candidate}\n");
      prop_assert!(validate(&layout).is_err());
    }
  }

  #[test]
  fn layouts_reject_oversized_keys(key in valid_key()) {
    // The generator stays well below the 128 byte limit, so padding pushes the
    // key over the limit without changing any other property.
    let oversized = format!("{key}{}", "A".repeat(128));
    let layout = format!("{oversized}=\n");
    prop_assert!(validate(&layout).is_err());
  }
}
