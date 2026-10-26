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

async fn put_versions(
  fixture: &Fixture,
  id: &str,
  values: &[&str],
) {
  for value in values {
    fixture
      .request(
        Method::PUT,
        &format!("/api/v1/environments/{id}/secrets/API_KEY"),
        Some(serde_json::json!({ "value": value })),
      )
      .await;
  }
}

async fn current_value(
  fixture: &Fixture,
  id: &str,
) -> serde_json::Value {
  fixture
    .request(
      Method::POST,
      &format!("/api/v1/environments/{id}/secrets/API_KEY/reveal"),
      None,
    )
    .await["value"]
    .clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn secret_history_pages_versions_without_values() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  put_versions(&fixture, &id, &["marker-one", "marker-two", "marker-three"]).await;

  let result = fixture
    .run(&[
      "secret",
      "history",
      "fixture/local",
      "API_KEY",
      "--limit",
      "2",
    ])
    .await;
  assert!(result.status.success());
  let stdout = String::from_utf8_lossy(&result.stdout);
  let stderr = String::from_utf8_lossy(&result.stderr);
  assert!(stdout.contains("VERSION"), "{stdout}");
  assert!(stdout.contains("STATUS"), "{stdout}");
  let rows: Vec<&str> = stdout
    .lines()
    .filter(|line| line.starts_with('v'))
    .collect();
  assert_eq!(rows.len(), 2, "{stdout}");
  assert!(
    rows[0].starts_with("v3") && rows[0].contains("current"),
    "{stdout}"
  );
  assert!(
    rows[1].starts_with("v2") && rows[1].contains("available"),
    "{stdout}"
  );
  assert!(rows[1].contains("updated") && rows[1].contains("admin@example.com"));
  assert!(!stdout.contains("marker-"), "{stdout}");
  assert!(
    stderr.contains("Next page: dopbase secret history fixture/local API_KEY --limit 2 --before 2"),
    "{stderr}"
  );

  let page = fixture
    .json(&["secret", "history", &id, "API_KEY", "--before", "2"])
    .await;
  assert_eq!(page["items"].as_array().unwrap().len(), 1);
  assert_eq!(page["items"][0]["version"], 1);
  assert_eq!(page["items"][0]["changeType"], "created");
  assert!(page["nextBefore"].is_null());

  let version = fixture
    .json(&["secret", "get", &id, "API_KEY", "--version", "2"])
    .await;
  assert_eq!(version["version"], 2);
  assert_eq!(version["current"], false);
  assert!(version.get("value").is_none());
  failure(
    &fixture
      .run(&["secret", "get", &id, "API_KEY", "--version", "9"])
      .await,
    "version 9 of API_KEY was not found",
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn secret_rollback_needs_an_explicit_version_and_yes_without_a_terminal() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  put_versions(&fixture, &id, &["marker-one", "marker-two", "marker-three"]).await;

  failure(
    &fixture.run(&["secret", "rollback", &id, "API_KEY"]).await,
    "pass VERSION when running non-interactively",
  );
  failure(
    &fixture
      .run(&["secret", "rollback", &id, "API_KEY", "--yes"])
      .await,
    "--yes needs a VERSION",
  );
  failure(
    &fixture
      .run(&["secret", "rollback", &id, "API_KEY", "1"])
      .await,
    "Pass --yes",
  );
  failure(
    &fixture
      .run(&["secret", "rollback", &id, "API_KEY", "3", "--yes"])
      .await,
    "v3 is already the current value of API_KEY",
  );
  failure(
    &fixture
      .run(&["secret", "rollback", &id, "API_KEY", "8", "--yes"])
      .await,
    "version 8 of API_KEY was not found",
  );
  assert_eq!(current_value(&fixture, &id).await, "marker-three");

  let rolled_back = fixture
    .json(&[
      "secret",
      "rollback",
      "fixture/local",
      "API_KEY",
      "1",
      "--yes",
    ])
    .await;
  assert_eq!(rolled_back["version"], 4);
  assert_eq!(rolled_back["sourceVersion"], 1);
  assert_eq!(rolled_back["previousVersion"], 3);
  assert_eq!(current_value(&fixture, &id).await, "marker-one");

  let result = fixture
    .run(&["secret", "rollback", &id, "API_KEY", "3", "--yes"])
    .await;
  assert!(result.status.success());
  let stdout = String::from_utf8_lossy(&result.stdout);
  assert!(
    stdout.contains("Current:  v4  rolled back from v1"),
    "{stdout}"
  );
  assert!(
    stdout.contains("saved as v5 with the value from v3"),
    "{stdout}"
  );
  assert!(
    stdout.contains("Rolled back API_KEY to the value from v3. Current version: v5."),
    "{stdout}"
  );
  assert!(!stdout.contains("marker-"), "{stdout}");
  assert_eq!(current_value(&fixture, &id).await, "marker-three");
}

#[tokio::test(flavor = "multi_thread")]
async fn secret_history_marks_versions_past_retention_as_expired() {
  let fixture = Fixture::with_config(|config| config.secret_history_versions = 2).await;
  let id = fixture.environment().await;
  put_versions(&fixture, &id, &["marker-one", "marker-two", "marker-three"]).await;

  let stdout = String::from_utf8_lossy(
    &fixture
      .run(&["secret", "history", &id, "API_KEY"])
      .await
      .stdout,
  )
  .into_owned();
  let v1 = stdout.lines().find(|line| line.starts_with("v1")).unwrap();
  assert!(v1.contains("expired"), "{stdout}");
  failure(
    &fixture
      .run(&["secret", "rollback", &id, "API_KEY", "1", "--yes"])
      .await,
    "past the retention limit",
  );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn secret_rollback_offers_restorable_versions_in_a_terminal() {
  use super::support::terminal::Terminal;
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  put_versions(&fixture, &id, &["marker-one", "marker-two", "marker-three"]).await;

  let mut command = fixture.command();
  command.args(["secret", "rollback", &id, "API_KEY"]);
  let mut terminal = Terminal::new(command);
  // The list starts at v2: the current version is not offered.
  terminal.reply("Roll back API_KEY to which version?", "\x1b[B\n");
  terminal.reply("Roll back API_KEY to v1?", "y\n");
  assert!(terminal.finish().success(), "{}", terminal.transcript);
  assert!(
    terminal
      .transcript
      .contains("saved as v4 with the value from v1")
  );
  assert!(!terminal.transcript.contains("marker-"));
  assert_eq!(current_value(&fixture, &id).await, "marker-one");

  let mut command = fixture.command();
  command.args(["secret", "rollback", &id, "API_KEY", "2"]);
  let mut terminal = Terminal::new(command);
  terminal.reply("Roll back API_KEY to v2?", "n\n");
  assert!(!terminal.finish().success());
  assert_eq!(current_value(&fixture, &id).await, "marker-one");
}

/// An account without a password confirms plaintext access by approving a
/// code in the Admin UI, where single sign-on works.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn reveal_asks_for_browser_confirmation_when_the_account_has_no_password() {
  use super::support::{browser_credentials, terminal::Terminal};
  use app::cli::session;
  use serde_json::{Value, json};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  fixture
    .request(
      reqwest::Method::PUT,
      &format!("/api/v1/environments/{id}/secrets/API_KEY"),
      Some(json!({"value":"browser-confirmed-marker"})),
    )
    .await;
  let (root_cookie, root_csrf) = browser_credentials(&fixture).await;
  let api = reqwest::Client::new();
  let member: Value = api
    .post(format!("{}/api/v1/users", fixture.url))
    .header("cookie", &root_cookie)
    .header("x-dopbase-csrf", &root_csrf)
    .json(&json!({"email":"member@example.com","password":"correct-horse-456","role":"member"}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .await
    .unwrap();
  api
    .put(format!("{}/api/v1/grants", fixture.url))
    .header("cookie", &root_cookie)
    .header("x-dopbase-csrf", &root_csrf)
    .json(&json!({"principalType":"user","principalId":member["data"]["id"],"scopeType":"environment","scope":id,"level":"read"}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
  let login = |kind: &'static str| {
    let api = api.clone();
    let url = fixture.url.clone();
    async move {
      api
        .post(format!("{url}/api/v1/auth/login"))
        .json(
          &json!({"email":"member@example.com","password":"correct-horse-456","sessionKind":kind}),
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
    }
  };
  let cli: Value = login("cli").await.json().await.unwrap();
  // The member's Admin UI tab, signed in while the account still had a
  // password; it approves the request later.
  let browser = login("browser").await;
  let cookie = browser
    .headers()
    .get("set-cookie")
    .unwrap()
    .to_str()
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_owned();
  let browser: Value = browser.json().await.unwrap();
  let csrf = browser["data"]["csrfToken"].as_str().unwrap().to_owned();
  session::save(
    &fixture.resolved(),
    cli["data"]["token"].as_str().unwrap(),
    Some("member@example.com"),
  )
  .unwrap();

  // Make the member an SSO-only account: no password, one linked identity.
  let pool = fixture.state.db.pool();
  let now = chrono::Utc::now().to_rfc3339();
  sqlx::query("UPDATE admins SET password_hash='' WHERE email='member@example.com'")
    .execute(pool)
    .await
    .unwrap();
  sqlx::query("INSERT INTO sso_providers(id,preset,display_name,issuer,client_id,client_secret_ciphertext,client_secret_nonce,enabled,verified_at,created_at,updated_at) VALUES('sso_t','generic','Acme SSO','https://idp.example.com','client',x'00',x'00',1,?,?,?)")
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
  sqlx::query("INSERT INTO admin_identities(id,admin_id,provider_id,issuer,subject,linked_via,created_at) SELECT 'idn_t',id,'sso_t','https://idp.example.com','member-subject','manual',? FROM admins WHERE email='member@example.com'")
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();

  let mut command = fixture.command();
  command.args(["secret", "get", &id, "API_KEY", "--reveal"]);
  let mut terminal = Terminal::new(command);
  terminal.reply("Waiting for approval", "");
  let code = regex::Regex::new(r"code=([A-Z]{4}-[A-Z]{4})")
    .unwrap()
    .captures(&terminal.transcript)
    .unwrap()[1]
    .to_owned();
  assert!(!terminal.transcript.contains("Password:"));
  api
    .post(format!("{}/api/v1/auth/cli/approve", fixture.url))
    .header("cookie", &cookie)
    .header("x-dopbase-csrf", &csrf)
    .json(&json!({"userCode":code}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap();
  assert!(terminal.finish().success(), "{}", terminal.transcript);
  assert!(terminal.transcript.contains("browser-confirmed-marker"));
}
