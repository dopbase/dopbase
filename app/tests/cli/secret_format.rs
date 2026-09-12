use app::{
  cli::secret_format::{SecretFormat, parse, render},
  models::SecretInput,
};
use std::path::Path;

fn entries(values: &[(&str, &str)]) -> Vec<SecretInput> {
  values
    .iter()
    .map(|(key, value)| SecretInput {
      key: (*key).into(),
      value: (*value).into(),
    })
    .collect()
}

#[test]
fn infers_formats_and_preserves_dotenv_fallbacks() {
  for path in [".env", ".env.production", "secrets", "secrets.txt"] {
    assert_eq!(
      SecretFormat::for_input(Path::new(path), None).unwrap(),
      SecretFormat::Dotenv
    );
  }
  for path in ["secrets.json", "SECRETS.JSON"] {
    assert_eq!(
      SecretFormat::for_input(Path::new(path), None).unwrap(),
      SecretFormat::Json
    );
  }
  for path in ["secrets.yaml", "secrets.yml", "SECRETS.YML"] {
    assert_eq!(
      SecretFormat::for_input(Path::new(path), None).unwrap(),
      SecretFormat::Yaml
    );
  }
}

#[test]
fn stdin_requires_an_explicit_format() {
  assert!(
    SecretFormat::for_input(Path::new("-"), None)
      .unwrap_err()
      .to_string()
      .contains("--format")
  );
  assert_eq!(
    SecretFormat::for_input(Path::new("-"), Some(SecretFormat::Json)).unwrap(),
    SecretFormat::Json
  );
}

#[test]
fn explicit_format_overrides_the_extension() {
  assert_eq!(
    SecretFormat::for_input(Path::new("secrets.json"), Some(SecretFormat::Yaml)).unwrap(),
    SecretFormat::Yaml
  );
  assert_eq!(
    SecretFormat::for_output(Some(Path::new("secrets.yaml")), Some(SecretFormat::Json)),
    SecretFormat::Json
  );
}

#[test]
fn parses_all_supported_formats() {
  assert_eq!(
    parse("A=one\nB=\"two three\"\n", SecretFormat::Dotenv)
      .unwrap()
      .len(),
    2
  );
  assert_eq!(
    parse(r#"{"A":"one","B":"two three"}"#, SecretFormat::Json)
      .unwrap()
      .len(),
    2
  );
  assert_eq!(
    parse("A: one\nB: \"two three\"\n", SecretFormat::Yaml)
      .unwrap()
      .len(),
    2
  );
}

#[test]
fn rejects_inputs_without_entries() {
  for (text, format) in [
    ("\n", SecretFormat::Dotenv),
    ("# comment\n", SecretFormat::Dotenv),
    ("{}", SecretFormat::Json),
    ("{}\n", SecretFormat::Yaml),
  ] {
    assert!(
      parse(text, format)
        .unwrap_err()
        .to_string()
        .contains("no secret entries")
    );
  }
}

#[test]
fn rejects_duplicate_keys() {
  for (text, format) in [
    (r#"{"A":"first","A":"second"}"#, SecretFormat::Json),
    ("A: first\nA: second\n", SecretFormat::Yaml),
  ] {
    let message = format!("{:#}", parse(text, format).unwrap_err());
    assert!(message.contains("A"), "{message}");
    assert!(!message.contains("first"), "{message}");
    assert!(!message.contains("second"), "{message}");
  }
}

#[test]
fn rejects_empty_keys_and_non_string_values_without_exposing_values() {
  for (text, format, key, kind) in [
    ("=marker\n", SecretFormat::Dotenv, "", "empty"),
    (r#"{"":"marker"}"#, SecretFormat::Json, "", "empty"),
    (r#"{"PORT":8840}"#, SecretFormat::Json, "PORT", "number"),
    ("PORT: 8840\n", SecretFormat::Yaml, "PORT", "number"),
    ("ENABLED: true\n", SecretFormat::Yaml, "ENABLED", "boolean"),
    (
      "NESTED:\n  TOKEN: marker\n",
      SecretFormat::Yaml,
      "NESTED",
      "object",
    ),
    ("ITEMS:\n  - marker\n", SecretFormat::Yaml, "ITEMS", "array"),
    ("NOTHING: null\n", SecretFormat::Yaml, "NOTHING", "null"),
  ] {
    let message = format!("{:#}", parse(text, format).unwrap_err());
    if !key.is_empty() {
      assert!(message.contains(key), "{message}");
    }
    assert!(message.contains(kind), "{message}");
    assert!(!message.contains("marker"), "{message}");
  }
}

#[test]
fn rejects_multiple_yaml_documents() {
  assert!(parse("A: one\n---\nB: two\n", SecretFormat::Yaml).is_err());
}

#[test]
fn renders_sorted_deterministic_output() {
  let values = entries(&[("Z_KEY", "last"), ("A_KEY", "true"), ("M_KEY", "two words")]);
  assert_eq!(
    render(&values, SecretFormat::Dotenv).unwrap(),
    "A_KEY=true\nM_KEY=\"two words\"\nZ_KEY=last\n"
  );
  assert_eq!(
    render(&values, SecretFormat::Json).unwrap(),
    "{\n  \"A_KEY\": \"true\",\n  \"M_KEY\": \"two words\",\n  \"Z_KEY\": \"last\"\n}\n"
  );
  let yaml = render(&values, SecretFormat::Yaml).unwrap();
  assert!(yaml.ends_with('\n'));
  assert!(yaml.find("A_KEY:").unwrap() < yaml.find("M_KEY:").unwrap());
  assert!(yaml.find("M_KEY:").unwrap() < yaml.find("Z_KEY:").unwrap());
  let reparsed = parse(&yaml, SecretFormat::Yaml).unwrap();
  assert_eq!(
    reparsed
      .iter()
      .map(|entry| (entry.key.as_str(), entry.value.as_str()))
      .collect::<Vec<_>>(),
    [("A_KEY", "true"), ("M_KEY", "two words"), ("Z_KEY", "last")]
  );
}
