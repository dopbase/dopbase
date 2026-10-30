use super::support::Fixture;

#[tokio::test(flavor = "multi_thread")]
async fn token_commands_forward_expiry_and_disclose_the_value_only_at_creation() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let created = fixture
    .json(&[
      "token",
      "create",
      "fixture/local",
      "--name",
      "deploy",
      "--role",
      "runner",
      "--expires-in",
      "12h",
    ])
    .await;
  let token_id = created["token"]["id"].as_str().unwrap();
  let plaintext = created["plaintextToken"].as_str().unwrap();
  assert!(plaintext.starts_with("dbs_"));
  assert_eq!(created["token"]["name"], "deploy");
  let expiry =
    chrono::DateTime::parse_from_rfc3339(created["token"]["expiresAt"].as_str().unwrap()).unwrap();
  let issued =
    chrono::DateTime::parse_from_rfc3339(created["token"]["createdAt"].as_str().unwrap()).unwrap();
  assert_eq!((expiry - issued).num_hours(), 12);
  let listed = fixture.json(&["token", "list", &id]).await;
  assert_eq!(listed[0]["id"], token_id);
  assert_eq!(listed[0]["expiresAt"], created["token"]["expiresAt"]);
  assert!(!listed.to_string().contains(plaintext));
  fixture.json(&["token", "revoke", token_id]).await;
  let listed = fixture.json(&["token", "list", &id]).await;
  assert!(listed[0]["revokedAt"].is_string());
  let human = fixture.run(&["token", "list", &id]).await;
  assert!(human.status.success());
  assert!(String::from_utf8_lossy(&human.stdout).contains("revoked"));
  assert!(!String::from_utf8_lossy(&human.stdout).contains(plaintext));
}

#[tokio::test(flavor = "multi_thread")]
async fn personal_tokens_act_for_their_account_until_revoked() {
  use super::support::{failure, output, success};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  // Root has no personal tokens.
  let root = fixture
    .run(&["token", "personal", "create", "--name", "mine"])
    .await;
  assert!(!root.status.success());
  fixture.sign_in_member().await;

  let created = fixture
    .json(&[
      "token",
      "personal",
      "create",
      "--name",
      "laptop-agent",
      "--level",
      "metadata",
      "--env",
      "fixture/local",
      "--expires-in",
      "12h",
    ])
    .await;
  let plaintext = created["plaintextToken"].as_str().unwrap().to_owned();
  assert!(plaintext.starts_with("dpp_"));
  assert_eq!(created["token"]["scopeLabel"], "fixture/local");
  let listed = fixture.json(&["token", "personal", "list"]).await;
  assert_eq!(listed[0]["name"], "laptop-agent");
  assert!(!listed.to_string().contains(&plaintext));

  // The token lists names but never reveals values.
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", &plaintext)
    .args(["--json", "secret", "list", &id]);
  success(&output(command, None).await);
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", &plaintext)
    .args(["export", &id, "--stdout"]);
  failure(
    &output(command, None).await,
    "Your access to this environment does not allow this.",
  );

  let token_id = created["token"]["id"].as_str().unwrap();
  fixture
    .json(&["token", "personal", "revoke", token_id])
    .await;
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", &plaintext)
    .args(["project", "list"]);
  failure(&output(command, None).await, "AUTHENTICATION_INVALID");
}
