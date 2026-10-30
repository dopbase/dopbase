use super::support::{Fixture, output, success};

#[tokio::test(flavor = "multi_thread")]
async fn access_shows_an_account_its_grants_and_a_token_its_limit() {
  let fixture = Fixture::new().await;
  fixture.environment().await;
  let root = fixture.json(&["access"]).await;
  assert_eq!(root["roleName"], "Root");
  assert_eq!(root["maxLevel"], "project_manager");

  let project = fixture.json(&["access", "--project", "fixture"]).await;
  assert_eq!(project["environments"][0]["name"], "local");

  fixture.sign_in_member().await;
  let member = fixture.json(&["access"]).await;
  assert_eq!(member["email"], "member@example.com");
  assert_eq!(member["grants"][0]["scopeLabel"], "fixture");
  assert_eq!(member["grants"][0]["level"], "read");
  assert!(member["token"].is_null());
  let human = fixture.run(&["access"]).await;
  assert!(human.status.success());
  let text = String::from_utf8_lossy(&human.stdout);
  assert!(text.contains("Member (up to manage)"), "{text}");
  assert!(text.contains("direct"), "{text}");

  let created = fixture
    .json(&[
      "token",
      "personal",
      "create",
      "--name",
      "agent",
      "--level",
      "metadata",
      "--project",
      "fixture",
    ])
    .await;
  let mut command = fixture.command();
  command
    .env("DOPBASE_TOKEN", created["plaintextToken"].as_str().unwrap())
    .args(["--json", "access"]);
  let through_token = success(&output(command, None).await);
  assert_eq!(through_token["token"]["name"], "agent");
  assert_eq!(through_token["token"]["level"], "metadata");
  assert_eq!(through_token["token"]["scopeLabel"], "fixture");
}
