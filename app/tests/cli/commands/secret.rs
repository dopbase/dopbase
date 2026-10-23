use super::support::{Fixture, failure, output, success};
use reqwest::Method;

#[tokio::test(flavor = "multi_thread")]
async fn secret_commands_preserve_stdin_and_keep_metadata_private() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let value = " first line \r\nsecond line\n";
  let mut command = fixture.command();
  command.args([
    "--json",
    "secret",
    "set",
    "fixture/local",
    "API_KEY",
    "--stdin",
  ]);
  let saved = success(&output(command, Some(value.as_bytes())).await);
  assert_eq!(saved["version"], 1);
  let revealed = fixture
    .request(
      Method::POST,
      &format!("/api/v1/environments/{id}/secrets/API_KEY/reveal"),
      None,
    )
    .await;
  assert_eq!(revealed["value"], value);
  let mut command = fixture.command();
  command.args(["--json", "secret", "set", &id, "API_KEY", "--stdin"]);
  assert_eq!(
    success(&output(command, Some(value.as_bytes())).await)["version"],
    2
  );
  let listed = fixture.json(&["secret", "list", &id]).await;
  assert_eq!(listed[0]["key"], "API_KEY");
  assert_eq!(listed[0]["version"], 2);
  assert!(!listed.to_string().contains("first line"));
  let metadata = fixture.json(&["secret", "get", &id, "API_KEY"]).await;
  assert_eq!(metadata["key"], "API_KEY");
  assert_eq!(metadata["version"], 2);
  assert!(!metadata.to_string().contains("first line"));
  failure(
    &fixture.run(&["secret", "set", &id, "API_KEY"]).await,
    "use --stdin",
  );
  failure(
    &fixture.run(&["secret", "delete", &id, "API_KEY"]).await,
    "Pass --yes",
  );
  fixture
    .json(&["secret", "delete", &id, "API_KEY", "--yes"])
    .await;
  assert!(
    fixture
      .json(&["secret", "list", &id])
      .await
      .as_array()
      .unwrap()
      .is_empty()
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn rotation_and_promotion_swap_the_two_values() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let mut command = fixture.command();
  command.args(["--json", "secret", "set", &id, "API_KEY", "--stdin"]);
  assert_eq!(
    success(&output(command, Some(b"before-marker")).await)["version"],
    1
  );
  let mut command = fixture.command();
  command.args([
    "--json", "secret", "rotate", &id, "API_KEY", "--stdin", "--grace", "2h",
  ]);
  let rotated = success(&output(command, Some(b"after-marker")).await);
  assert_eq!(rotated["version"], 2);
  assert_eq!(rotated["previousVersion"], 1);
  assert!(rotated["previousExpiresAt"].is_string());
  // Metadata names the rotation but never a value.
  let metadata = fixture.json(&["secret", "get", &id, "API_KEY"]).await;
  assert_eq!(metadata["previousVersion"], 1);
  assert!(!metadata.to_string().contains("before-marker"));
  assert!(!metadata.to_string().contains("after-marker"));
  let listed = fixture.json(&["secret", "list", &id]).await;
  assert_eq!(listed[0]["previousVersion"], 1);
  assert!(!listed.to_string().contains("before-marker"));
  assert!(!listed.to_string().contains("after-marker"));
  // Promote rolls the secret back to the value the rotation replaced.
  let promoted = fixture
    .json(&["secret", "promote", &id, "API_KEY", "--yes"])
    .await;
  assert_eq!(promoted["version"], 1);
  assert_eq!(promoted["previousVersion"], 2);
  assert_eq!(promoted["previousExpiresAt"], rotated["previousExpiresAt"]);
  failure(
    &fixture.run(&["secret", "promote", &id, "API_KEY"]).await,
    "Pass --yes",
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn rotation_validates_the_grace_period_and_the_target_secret() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  failure(
    &fixture.run(&["secret", "rotate", &id, "API_KEY"]).await,
    "use --stdin",
  );
  failure(
    &fixture
      .run(&[
        "secret", "rotate", &id, "API_KEY", "--stdin", "--grace", "30m",
      ])
      .await,
    "hours or days",
  );
  failure(
    &fixture
      .run(&[
        "secret", "rotate", &id, "API_KEY", "--stdin", "--grace", "31d",
      ])
      .await,
    "up to 30 days",
  );
  let mut command = fixture.command();
  command.args(["secret", "rotate", &id, "MISSING_KEY", "--stdin"]);
  let missing = output(command, Some(b"new-marker")).await;
  assert!(!missing.status.success());
  assert!(String::from_utf8_lossy(&missing.stderr).contains("not found"));
  // A zero grace keeps no previous value, so nothing can be promoted back.
  let mut command = fixture.command();
  command.args(["--json", "secret", "set", &id, "API_KEY", "--stdin"]);
  success(&output(command, Some(b"immediate-marker")).await);
  let mut command = fixture.command();
  command.args([
    "--json", "secret", "rotate", &id, "API_KEY", "--stdin", "--grace", "0",
  ]);
  let rotated = success(&output(command, Some(b"replacement-marker")).await);
  assert_eq!(rotated["version"], 2);
  assert!(rotated.get("previousVersion").is_none());
  assert!(rotated.get("previousExpiresAt").is_none());
  failure(
    &fixture
      .run(&["secret", "promote", &id, "API_KEY", "--yes"])
      .await,
    "no previous value",
  );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn reveal_prints_the_previous_value_for_verification() {
  use super::support::{PASSWORD, terminal::Terminal};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let mut command = fixture.command();
  command.args(["--json", "secret", "set", &id, "API_KEY", "--stdin"]);
  success(&output(command, Some(b"before-marker")).await);
  let mut command = fixture.command();
  command.args(["--json", "secret", "rotate", &id, "API_KEY", "--stdin"]);
  success(&output(command, Some(b"after-marker")).await);
  let mut command = fixture.command();
  command.args([
    "secret",
    "get",
    &id,
    "API_KEY",
    "--reveal",
    "--include-previous",
  ]);
  let mut terminal = Terminal::new(command);
  terminal.reply("Password:", &format!("{PASSWORD}\n"));
  assert!(terminal.finish().success());
  assert!(terminal.transcript.contains("Current:"));
  assert!(terminal.transcript.contains("after-marker"));
  assert!(terminal.transcript.contains("Previous:"));
  assert!(terminal.transcript.contains("before-marker"));
  assert!(terminal.transcript.contains("Previous valid until:"));
  assert!(!terminal.transcript.contains(PASSWORD));
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn masked_secret_input_and_reveal_use_the_terminal() {
  use super::support::{PASSWORD, terminal::Terminal};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let mut command = fixture.command();
  command.args(["secret", "set", &id, "API_KEY"]);
  let mut terminal = Terminal::new(command);
  terminal.reply("Secret value:", "masked-secret-marker\n");
  assert!(terminal.finish().success());
  assert!(!terminal.transcript.contains("masked-secret-marker"));
  let mut command = fixture.command();
  command.args(["secret", "get", &id, "API_KEY", "--reveal"]);
  let mut terminal = Terminal::new(command);
  terminal.reply("Password:", &format!("{PASSWORD}\n"));
  assert!(terminal.finish().success());
  assert!(terminal.transcript.contains("masked-secret-marker"));
  assert!(!terminal.transcript.contains(PASSWORD));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_metadata_secret_commands_return_metadata_without_values() {
  use super::support::{Fixture, output, success};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  fixture
    .request(
      reqwest::Method::PUT,
      &format!("/api/v1/environments/{id}/secrets/API_KEY"),
      Some(serde_json::json!({"value":"agent-secret-private-marker"})),
    )
    .await;
  let created = fixture.agent_token().await;
  let token = created["plaintextToken"].as_str().unwrap();
  for arguments in [
    vec!["secret", "list", "fixture/local"],
    vec!["secret", "get", &id, "API_KEY"],
  ] {
    let mut command = fixture.command();
    command
      .env("DOPBASE_TOKEN", token)
      .args(["--json"])
      .args(arguments);
    let result = output(command, None).await;
    let value = success(&result);
    let metadata = if value.is_array() { &value[0] } else { &value };
    assert_eq!(metadata["key"], "API_KEY");
    assert_eq!(metadata["version"], 1);
    assert!(metadata["createdAt"].is_string());
    assert!(metadata["updatedAt"].is_string());
    assert!(metadata.get("value").is_none());
    assert!(!String::from_utf8_lossy(&result.stdout).contains("agent-secret-private-marker"));
    assert!(!String::from_utf8_lossy(&result.stderr).contains("agent-secret-private-marker"));
    assert!(!String::from_utf8_lossy(&result.stdout).contains(token));
  }
}
