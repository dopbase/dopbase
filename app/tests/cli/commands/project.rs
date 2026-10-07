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

#[tokio::test(flavor = "multi_thread")]
async fn agent_metadata_project_commands_use_environment_and_saved_credentials() {
  use super::support::{output, success};
  let fixture = Fixture::new().await;
  let environment = fixture.environment().await;
  let created = fixture.agent_token().await;
  let token = created["plaintextToken"].as_str().unwrap();
  for saved in [false, true] {
    if saved {
      app::cli::session::save(&fixture.resolved(), token, None).unwrap();
    }
    for arguments in [
      ["project", "list"].as_slice(),
      ["project", "show", "fixture"].as_slice(),
    ] {
      let mut command = fixture.command();
      if !saved {
        command.env("DOPBASE_TOKEN", token);
      }
      command.args(["--json"]).args(arguments);
      let result = output(command, None).await;
      let value = success(&result);
      assert_eq!(
        if value.is_array() {
          &value[0]["name"]
        } else {
          &value["name"]
        },
        "fixture"
      );
      assert!(!String::from_utf8_lossy(&result.stdout).contains(token));
    }
  }
  assert!(!environment.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_metadata_rejects_invalid_expired_and_revoked_tokens_without_fallback() {
  use super::support::{failure, output};
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let created = fixture.agent_token().await;
  let token = created["plaintextToken"].as_str().unwrap();
  let token_id = created["token"]["id"].as_str().unwrap();
  for scenario in ["invalid", "expired", "revoked"] {
    let credential = if scenario == "invalid" {
      "dpa_unknown_credential"
    } else {
      token
    };
    let column = if scenario == "expired" {
      "expires_at"
    } else {
      "revoked_at"
    };
    if scenario != "invalid" {
      sqlx::query(&format!("UPDATE agent_tokens SET {column}=? WHERE id=?"))
        .bind((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339())
        .bind(token_id)
        .execute(fixture.state.db.pool())
        .await
        .unwrap();
    }
    for arguments in [
      vec!["project", "list"],
      vec!["project", "show", "fixture"],
      vec!["env", "list", "fixture"],
      vec!["env", "show", &id],
      vec!["secret", "list", &id],
      vec!["secret", "get", &id, "API_KEY"],
    ] {
      let mut command = fixture.command();
      command.env("DOPBASE_TOKEN", credential).args(arguments);
      let result = output(command, None).await;
      failure(&result, "AUTHENTICATION_INVALID");
      assert!(result.stdout.is_empty());
      assert!(!String::from_utf8_lossy(&result.stderr).contains(credential));
    }
    if scenario != "invalid" {
      sqlx::query(&format!("UPDATE agent_tokens SET {column}=NULL WHERE id=?"))
        .bind(token_id)
        .execute(fixture.state.db.pool())
        .await
        .unwrap();
    }
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_metadata_requests_skip_the_human_session_preflight() {
  use super::support::{command, output, serve, success};
  use axum::{
    Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get,
  };
  use serde_json::json;
  use std::sync::{Arc, Mutex};
  async fn session(State(requests): State<Arc<Mutex<Vec<&'static str>>>>) -> impl IntoResponse {
    requests.lock().unwrap().push("session");
    (
      StatusCode::FORBIDDEN,
      Json(json!({"error":{"AUTHORIZATION_DENIED":"Human sessions only."}})),
    )
  }
  async fn projects(
    State(requests): State<Arc<Mutex<Vec<&'static str>>>>
  ) -> Json<serde_json::Value> {
    requests.lock().unwrap().push("projects");
    Json(json!({"data":[{"id":"prj_fixture","name":"fixture"}]}))
  }
  let requests = Arc::new(Mutex::new(Vec::new()));
  let router = Router::new()
    .route("/api/v1/auth/session", get(session))
    .route("/api/v1/projects", get(projects))
    .with_state(requests.clone());
  let (url, server) = serve(router).await;
  let directory = tempfile::TempDir::new().unwrap();
  let mut cmd = command(directory.path());
  cmd
    .env("DOPBASE_TOKEN", "dpa_fixture_metadata")
    .args(["--server", &url, "--json", "project", "list"]);
  let result = output(cmd, None).await;
  server.abort();
  assert_eq!(success(&result)[0]["name"], "fixture");
  assert_eq!(*requests.lock().unwrap(), ["projects"]);
}
