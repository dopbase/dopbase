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
