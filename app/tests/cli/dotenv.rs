use app::cli::dotenv::parse;

#[test]
fn parses_without_expansion() {
  let values = parse("A=$UNSET_VARIABLE\nB=\"two three\"\nEMPTY=\n# ignored\n").unwrap();
  assert_eq!(values.len(), 3);
  assert_eq!(values[0].value, "$UNSET_VARIABLE");
  assert_eq!(values[1].value, "two three");
  assert_eq!(values[2].value, "");
}

#[test]
fn rejects_duplicates() {
  assert!(parse("A=1\nA=2").is_err());
}

#[test]
fn editor_round_trips_values_and_preserves_value_free_layout() {
  use app::{
    cli::dotenv::{parse_document, render, render_layout, validate_layout},
    models::SecretInput,
  };
  let values: Vec<_> = [
    "",
    "a b",
    "first\nsecond\r\n",
    "a\tb",
    "quote\" and \\ slash",
    "$HOME $(touch /tmp/do-not-run)",
    "literal\\n",
    "#secret",
  ]
  .into_iter()
  .enumerate()
  .map(|(index, value)| SecretInput {
    key: format!("KEY_{index}"),
    value: value.into(),
  })
  .collect();
  let rendered = render(&values);
  let parsed = parse(&rendered).unwrap();
  assert_eq!(
    parsed.iter().map(|entry| &entry.value).collect::<Vec<_>>(),
    values.iter().map(|entry| &entry.value).collect::<Vec<_>>()
  );
  let text = "# app\nexport A=\"private value\"\n\nB=second # label\n";
  let (entries, layout) = parse_document(text).unwrap();
  assert_eq!(entries[0].value, "private value");
  assert_eq!(entries[1].value, "second");
  assert_eq!(layout, "# app\nexport A=\n\n# label\nB=\n");
  assert!(!layout.contains("private value"));
  validate_layout(&layout).unwrap();
  let merged = render_layout(
    Some(&layout),
    &[
      SecretInput {
        key: "B".into(),
        value: "current".into(),
      },
      SecretInput {
        key: "C".into(),
        value: "new".into(),
      },
    ],
  )
  .unwrap();
  assert!(!merged.contains("A="));
  assert!(merged.contains("B=current\nC=new\n"));
  for invalid in [
    "A=secret",
    "A=\"\"",
    "A= # comment",
    "A=\nA=",
    "not a slot",
    "1A=",
  ] {
    assert!(validate_layout(invalid).is_err(), "accepted invalid layout");
  }
}

#[test]
fn editor_parser_rejects_malformed_input_without_echoing_it() {
  for input in [
    "bad-private-marker=secret",
    "A=\"private-marker",
    "A=\"private\"marker\"",
    "A='private'marker'",
    "private-marker",
  ] {
    let error = parse(input).unwrap_err().to_string();
    assert!(error.contains("line 1"));
    assert!(!error.contains("private"));
  }
}
