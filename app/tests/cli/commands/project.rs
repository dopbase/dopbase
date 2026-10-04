use super::support::{Fixture, failure};
use reqwest::Method;

#[tokio::test(flavor = "multi_thread")]
async fn project_commands_preserve_identity_and_require_delete_confirmation() {
  let fixture = Fixture::new().await;
  let created = fixture.json(&["project", "create", "billing"]).await;
  let id = created["id"].as_str().unwrap();
  assert_eq!(created["name"], "billing");
  assert_eq!(
    fixture.json(&["project", "show", id]).await["name"],
    "billing"
  );
  let listed = fixture.json(&["project", "list"]).await;
  assert_eq!(listed.as_array().unwrap().len(), 1);
  assert_eq!(listed[0]["id"], id);
  let renamed = fixture
    .json(&["project", "rename", "billing", "payments"])
    .await;
  assert_eq!(renamed["id"], id);
  assert_eq!(renamed["name"], "payments");
  failure(
    &fixture.run(&["project", "delete", "payments"]).await,
    "Pass --yes",
  );
  assert_eq!(
    fixture
      .request(Method::GET, "/api/v1/projects", None)
      .await
      .as_array()
      .unwrap()
      .len(),
    1
  );
  let deleted = fixture
    .json(&["project", "delete", "payments", "--yes"])
    .await;
  assert_eq!(deleted["affected"]["projects"], 1);
  assert!(
    fixture
      .json(&["project", "list"])
      .await
      .as_array()
      .unwrap()
      .is_empty()
  );
}
