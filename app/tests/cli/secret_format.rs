use app::{
  cli::secret_format::{ExportFormat, SecretFormat, parse, render},
  models::SecretInput,
};
use std::{
  io::Write,
  path::Path,
  process::{Command, Stdio},
};

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
    ExportFormat::for_output(Some(Path::new("secrets.yaml")), Some(ExportFormat::Json)),
    ExportFormat::Json
  );
  assert_eq!(
    ExportFormat::for_output(Some(Path::new("secrets.json")), None),
    ExportFormat::Json
  );
  assert_eq!(
    ExportFormat::for_output(Some(Path::new(".env.production")), None),
    ExportFormat::Dotenv
  );
  assert_eq!(ExportFormat::for_output(None, None), ExportFormat::Dotenv);
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
    render(&values, ExportFormat::Dotenv).unwrap(),
    "A_KEY=true\nM_KEY=\"two words\"\nZ_KEY=last\n"
  );
  assert_eq!(
    render(&values, ExportFormat::Json).unwrap(),
    "{\n  \"A_KEY\": \"true\",\n  \"M_KEY\": \"two words\",\n  \"Z_KEY\": \"last\"\n}\n"
  );
  let yaml = render(&values, ExportFormat::Yaml).unwrap();
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

#[test]
fn renders_docker_values_without_quotes_or_interpolation() {
  let values = entries(&[
    ("TRAILING", "value "),
    ("UNICODE", "你好"),
    ("QUOTED", "say \"hello\""),
    ("HASH", "#fragment"),
    ("EMPTY", ""),
    ("DOLLAR", "$HOME"),
    ("DB_HOST", "YOUR VALUE HAS SPACE"),
    ("BACKSLASH", r"C:\workspace"),
    ("APP_SECRET", "==aHR0cHM6Ly9kb3BiYXNlLmNvbS8"),
    ("LEADING", " value"),
  ]);
  assert_eq!(
    render(&values, ExportFormat::Docker).unwrap(),
    concat!(
      "APP_SECRET===aHR0cHM6Ly9kb3BiYXNlLmNvbS8\n",
      "BACKSLASH=C:\\workspace\n",
      "DB_HOST=YOUR VALUE HAS SPACE\n",
      "DOLLAR=$HOME\n",
      "EMPTY=\n",
      "HASH=#fragment\n",
      "LEADING= value\n",
      "QUOTED=say \"hello\"\n",
      "TRAILING=value \n",
      "UNICODE=你好\n",
    )
  );
}

#[test]
fn rejects_values_docker_cannot_represent_without_exposing_them() {
  for value in ["private\nmarker", "private\rmarker", "private\0marker"] {
    let message = render(&entries(&[("SECRET_KEY", value)]), ExportFormat::Docker)
      .unwrap_err()
      .to_string();
    assert!(message.contains("SECRET_KEY"), "{message}");
    assert!(!message.contains("private"), "{message}");
    assert!(!message.contains("marker"), "{message}");
  }

  let accepted = "x".repeat(65_533);
  assert!(render(&entries(&[("A", accepted.as_str())]), ExportFormat::Docker).is_ok());

  let oversized = "private".repeat(9_363);
  let message = render(
    &entries(&[("SECRET_KEY", oversized.as_str())]),
    ExportFormat::Docker,
  )
  .unwrap_err()
  .to_string();
  assert!(message.contains("SECRET_KEY"), "{message}");
  assert!(!message.contains("private"), "{message}");
}

struct DockerContainer(String);

impl Drop for DockerContainer {
  fn drop(&mut self) {
    let _ = Command::new("docker")
      .args(["stop", "--time=0", &self.0])
      .output();
  }
}

#[test]
#[ignore = "requires a local Docker engine and the alpine:3.22 image"]
fn docker_format_round_trips_through_docker_exec() {
  let values = entries(&[
    ("TRAILING", "value "),
    ("QUOTED", "say \"hello\""),
    ("HASH", "#fragment"),
    ("EMPTY", ""),
    ("DOLLAR", "$HOME"),
    ("DB_HOST", "YOUR VALUE HAS SPACE"),
    ("BACKSLASH", r"C:\workspace"),
    ("APP_SECRET", "==aHR0cHM6Ly9kb3BiYXNlLmNvbS8"),
    ("LEADING", " value"),
  ]);
  let rendered = render(&values, ExportFormat::Docker).unwrap();
  let name = format!("dopbase-docker-export-test-{}", std::process::id());
  let started = Command::new("docker")
    .args([
      "run",
      "--detach",
      "--rm",
      "--name",
      &name,
      "alpine:3.22",
      "sleep",
      "60",
    ])
    .output()
    .unwrap();
  assert!(
    started.status.success(),
    "failed to start Docker test container: {}",
    String::from_utf8_lossy(&started.stderr)
  );
  let _container = DockerContainer(name.clone());

  let mut child = Command::new("docker")
    .args([
      "exec",
      "--env-file=/dev/stdin",
      &name,
      "sh",
      "-ec",
      concat!(
        "[ \"$APP_SECRET\" = \"==aHR0cHM6Ly9kb3BiYXNlLmNvbS8\" ]\n",
        "[ \"$BACKSLASH\" = 'C:\\workspace' ]\n",
        "[ \"$DB_HOST\" = 'YOUR VALUE HAS SPACE' ]\n",
        "[ \"$DOLLAR\" = '$HOME' ]\n",
        "[ -z \"$EMPTY\" ]\n",
        "[ \"$HASH\" = '#fragment' ]\n",
        "[ \"$LEADING\" = ' value' ]\n",
        "[ \"$QUOTED\" = 'say \"hello\"' ]\n",
        "[ \"$TRAILING\" = 'value ' ]\n",
      ),
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  child
    .stdin
    .take()
    .unwrap()
    .write_all(rendered.as_bytes())
    .unwrap();
  let result = child.wait_with_output().unwrap();
  assert!(
    result.status.success(),
    "Docker did not preserve the rendered values: {}",
    String::from_utf8_lossy(&result.stderr)
  );
}
