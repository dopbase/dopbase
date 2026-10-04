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
