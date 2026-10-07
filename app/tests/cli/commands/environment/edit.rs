use super::super::support::{Fixture, PASSWORD, failure, terminal::Terminal};
use reqwest::Method;
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn editor_batches_preview_confirm_and_clean_up_without_disclosing_values() {
  for case in [
    "apply",
    "unchanged",
    "layout-only",
    "dry-run",
    "cancel",
    "invalid",
    "failure",
    "delete-all",
    "decline-delete-all",
    "edit-again",
  ] {
    let fixture = Fixture::new().await;
    let id = fixture.environment().await;
    let base = format!("/api/v1/environments/{id}/secrets");
    fixture
      .request(
        Method::PUT,
        &format!("{base}/OLD"),
        Some(json!({"value":"original-private-marker"})),
      )
      .await;
    if case == "unchanged" {
      fixture
        .request(
          Method::POST,
          &format!("{base}/import"),
          Some(json!({"mode":"merge","entries":[],"envLayout":"OLD=\n"})),
        )
        .await;
    }
    let executable = fixture.directory.path().join("editor");
    let marker = fixture.directory.path().join("editor-path");
    let content = match case {
      "invalid" => "A=\"invalid-private-marker",
      "unchanged" => "OLD=original-private-marker\n",
      "layout-only" => "# updated comment\nOLD=original-private-marker\n",
      "delete-all" | "decline-delete-all" => "",
      _ => "# app\nNEW=edited-private-marker\n",
    };
    fs::write(&executable, format!(
      "#!/bin/sh\numask 077\ntest -z \"$DOPBASE_TOKEN$UNRELATED_CREDENTIAL\" || exit 42\nprintf '%s' '{content}' > \"$1\"\nprintf '%s' \"$1\" > '{}'\nexit {}\n",
      marker.display(), if case == "failure" { 1 } else { 0 }
    )).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = fixture.command();
    command
      .env("DOPBASE_TOKEN", &fixture.token)
      .env("UNRELATED_CREDENTIAL", "credential-private-marker")
      .args(["env", "edit", "fixture/local", "--editor"])
      .arg(&executable);
    if case == "dry-run" {
      command.arg("--dry-run");
    }
    let mut terminal = Terminal::new(command);
    terminal.reply("Open secrets with this custom editor command?", "y\n");
    terminal.reply("Password:", &format!("{PASSWORD}\n"));
    match case {
      "failure" | "dry-run" | "unchanged" => {}
      "invalid" => terminal.reply("Edit again?", "n\n"),
      "cancel" => terminal.reply("(a)Apply, (e)edit again, or (c)cancel?", "\n"),
      "delete-all" | "decline-delete-all" => {
        terminal.reply("(a)Apply, (e)edit again, or (c)cancel?", "apply\n");
        terminal.reply(
          "Type fixture/local",
          if case == "delete-all" {
            "fixture/local\n"
          } else {
            "wrong\n"
          },
        );
      }
      "edit-again" => {
        terminal.reply("(a)Apply, (e)edit again, or (c)cancel?", "edit\n");
        terminal.reply("(a)Apply, (e)edit again, or (c)cancel?", "apply\n");
      }
      _ => terminal.reply("(a)Apply, (e)edit again, or (c)cancel?", "apply\n"),
    }
    let status = terminal.finish();
    assert_eq!(
      status.success(),
      matches!(
        case,
        "apply" | "dry-run" | "delete-all" | "edit-again" | "unchanged" | "layout-only"
      ),
      "{case}: {}",
      terminal.transcript
    );
    for value in [
      "original-private-marker",
      "edited-private-marker",
      "invalid-private-marker",
      "credential-private-marker",
      PASSWORD,
    ] {
      assert!(
        !terminal.transcript.contains(value),
        "{case} disclosed a value outside the editor"
      );
    }
    let path = fs::read_to_string(marker).unwrap();
    assert!(
      !std::path::Path::new(&path).parent().unwrap().exists(),
      "session was not cleaned up"
    );
    let keys = fixture.json(&["secret", "list", &id]).await;
    match case {
      "apply" | "edit-again" => {
        assert_eq!(keys[0]["key"], "NEW");
        assert_eq!(keys.as_array().unwrap().len(), 1);
        let layout = fixture
          .request(Method::GET, &format!("{base}/layout"), None)
          .await;
        assert_eq!(layout["layout"], "# app\nNEW=\n");
        let exported = fixture
          .request(Method::POST, &format!("{base}/export"), None)
          .await;
        assert_eq!(exported["entries"][0]["value"], "edited-private-marker");
      }
      "layout-only" | "unchanged" => {
        assert_eq!(keys[0]["key"], "OLD");
        assert_eq!(keys[0]["version"], 1);
        if case == "unchanged" {
          assert!(terminal.transcript.contains("No changes to save."));
        } else {
          assert!(terminal.transcript.contains("changed"));
        }
      }
      "delete-all" => assert!(keys.as_array().unwrap().is_empty()),
      _ => assert_eq!(keys[0]["key"], "OLD"),
    }
    fixture.stop().await;
  }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn edits_reject_changes_made_while_the_editor_is_open() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let base = format!("/api/v1/environments/{id}/secrets");
  let executable = fixture.directory.path().join("editor");
  let marker = fixture.directory.path().join("editor-path");
  fs::write(&executable, format!(
    "#!/bin/sh\numask 077\nprintf 'A=edited-private-marker\\n' > \"$1\"\nprintf '%s' \"$1\" > '{}'\ncurl --fail --silent -X PUT -H 'Authorization: Bearer {}' -H 'Content-Type: application/json' --data '{{\"value\":\"concurrent-private-marker\"}}' '{}{base}/CONCURRENT' > /dev/null\n",
    marker.display(), fixture.token, fixture.url
  )).unwrap();
  fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
  let mut command = fixture.command();
  command
    .args(["env", "edit", &id, "--editor"])
    .arg(&executable);
  let mut terminal = Terminal::new(command);
  terminal.reply("No secrets yet. Add some?", "y\n");
  terminal.reply("Open secrets with this custom editor command?", "y\n");
  terminal.reply("Password:", &format!("{PASSWORD}\n"));
  assert!(!terminal.finish().success());
  assert!(terminal.transcript.contains("IMPORT_PREVIEW_STALE"));
  assert!(!terminal.transcript.contains("edited-private-marker"));
  assert_eq!(
    fixture.json(&["secret", "list", &id]).await[0]["key"],
    "CONCURRENT"
  );
  let path = fs::read_to_string(marker).unwrap();
  assert!(!std::path::Path::new(&path).parent().unwrap().exists());
  fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn editor_rejects_automation_and_acknowledgement_cancellation_before_plaintext_access() {
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  failure(
    &fixture.run(&["env", "edit", &id]).await,
    "interactive terminal",
  );
  failure(
    &fixture.run(&["--json", "env", "edit", &id]).await,
    "cannot be combined",
  );
  let mut command = fixture.command();
  command.args(["env", "edit", &id, "--editor", "/bin/sh"]);
  let mut terminal = Terminal::new(command);
  terminal.reply("No secrets yet. Add some?", "y\n");
  terminal.reply("Open secrets with this custom editor command?", "n\n");
  assert!(!terminal.finish().success());
  let count: i64 =
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='secret.exported'")
      .fetch_one(fixture.state.db.pool())
      .await
      .unwrap();
  assert_eq!(count, 0);
  fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn termination_stops_the_editor_and_cleans_up_even_during_confirmation() {
  for phase in ["editor", "confirmation"] {
    let fixture = Fixture::new().await;
    let id = fixture.environment().await;
    let executable = fixture.directory.path().join("editor");
    let marker = fixture.directory.path().join("editor-path");
    let pid_file = fixture.directory.path().join("cli-pid");
    fs::write(&executable, format!(
      "#!/bin/sh\numask 077\nprintf '%s' \"$1\" > '{}'\nprintf '%s' \"$PPID\" > '{}'\nprintf 'A=interrupted-private-marker\\n' > \"$1\"\n{}\n",
      marker.display(), pid_file.display(), if phase == "editor" { "printf 'Editor ready\\n'\nexec sleep 30" } else { "exit 0" }
    )).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = fixture.command();
    command
      .args(["env", "edit", &id, "--editor"])
      .arg(&executable);
    let mut terminal = Terminal::new(command);
    terminal.reply("No secrets yet. Add some?", "y\n");
    terminal.reply("Open secrets with this custom editor command?", "y\n");
    terminal.reply("Password:", &format!("{PASSWORD}\n"));
    terminal.reply(
      if phase == "editor" {
        "Editor ready"
      } else {
        "(a)Apply, (e)edit again, or (c)cancel?"
      },
      "",
    );
    let pid = fs::read_to_string(pid_file)
      .unwrap()
      .parse::<i32>()
      .unwrap();
    nix::sys::signal::kill(
      nix::unistd::Pid::from_raw(pid),
      nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    assert_eq!(
      terminal.finish().code(),
      Some(130),
      "{phase}: {}",
      terminal.transcript
    );
    assert!(!terminal.transcript.contains("interrupted-private-marker"));
    let path = fs::read_to_string(marker).unwrap();
    assert!(!std::path::Path::new(&path).parent().unwrap().exists());
    assert!(
      fixture
        .json(&["secret", "list", &id])
        .await
        .as_array()
        .unwrap()
        .is_empty()
    );
    fixture.stop().await;
  }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn interruption_after_confirming_apply_waits_for_the_save_result() {
  use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{StatusCode, header},
  };
  use std::sync::Arc;
  use tokio::sync::Notify;
  #[derive(Clone)]
  struct Proxy {
    url: String,
    started: Arc<Notify>,
    release: Arc<Notify>,
  }
  async fn forward(
    State(state): State<Proxy>,
    request: Request,
  ) -> (StatusCode, Json<serde_json::Value>) {
    let method = request.method().clone();
    let uri = request.uri().to_string();
    let authorization = request
      .headers()
      .get(header::AUTHORIZATION)
      .cloned()
      .unwrap();
    let body = to_bytes(request.into_body(), 8 * 1024 * 1024)
      .await
      .unwrap();
    if uri.ends_with("/import")
      && serde_json::from_slice::<serde_json::Value>(&body).unwrap()["dryRun"] == false
    {
      state.started.notify_one();
      state.release.notified().await;
    }
    let mut forwarded = reqwest::Client::new()
      .request(method, format!("{}{uri}", state.url))
      .header(header::AUTHORIZATION, authorization);
    if !body.is_empty() {
      forwarded = forwarded
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);
    }
    let response = forwarded.send().await.unwrap();
    (response.status(), Json(response.json().await.unwrap()))
  }
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  let proxy = Proxy {
    url: fixture.url.clone(),
    started: Arc::new(Notify::new()),
    release: Arc::new(Notify::new()),
  };
  let (url, task) =
    super::super::support::serve(Router::new().fallback(forward).with_state(proxy.clone())).await;
  let mut resolved = fixture.resolved();
  resolved.url = url.clone();
  app::cli::session::save(&resolved, &fixture.token, None).unwrap();
  let executable = fixture.directory.path().join("editor");
  let pid_file = fixture.directory.path().join("cli-pid");
  let marker = fixture.directory.path().join("editor-path");
  fs::write(&executable, format!("#!/bin/sh\numask 077\nprintf '%s' \"$PPID\" > '{}'\nprintf '%s' \"$1\" > '{}'\nprintf 'A=confirmed-private-marker\\n' > \"$1\"\n", pid_file.display(), marker.display())).unwrap();
  fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
  let mut command = super::super::support::command(&fixture.client_dir);
  command
    .args(["--server", &url, "env", "edit", &id, "--editor"])
    .arg(&executable);
  let mut terminal = Terminal::new(command);
  terminal.reply("No secrets yet. Add some?", "y\n");
  terminal.reply("Open secrets with this custom editor command?", "y\n");
  terminal.reply("Password:", &format!("{PASSWORD}\n"));
  terminal.reply("(a)Apply, (e)edit again, or (c)cancel?", "apply\n");
  tokio::time::timeout(std::time::Duration::from_secs(10), proxy.started.notified())
    .await
    .unwrap();
  let pid = fs::read_to_string(pid_file)
    .unwrap()
    .parse::<i32>()
    .unwrap();
  nix::sys::signal::kill(
    nix::unistd::Pid::from_raw(pid),
    nix::sys::signal::Signal::SIGTERM,
  )
  .unwrap();
  terminal.reply("Waiting for the server response.", "");
  let editor_path = fs::read_to_string(&marker).unwrap();
  assert!(
    !std::path::Path::new(&editor_path)
      .parent()
      .unwrap()
      .exists()
  );
  proxy.release.notify_one();
  assert!(terminal.finish().success(), "{}", terminal.transcript);
  assert!(!terminal.transcript.contains("confirmed-private-marker"));
  let values = fixture
    .request(
      Method::POST,
      &format!("/api/v1/environments/{id}/secrets/export"),
      None,
    )
    .await;
  assert_eq!(values["entries"][0]["value"], "confirmed-private-marker");
  let path = fs::read_to_string(marker).unwrap();
  assert!(!std::path::Path::new(&path).parent().unwrap().exists());
  task.abort();
  fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn missing_targets_and_declined_empty_environments_stop_before_plaintext_confirmation() {
  let fixture = Fixture::new().await;
  fixture.environment().await;
  for target in ["absent/local", "fixture/missing", "fixture/local"] {
    let mut command = fixture.command();
    // An invalid editor must not prevent checking the target or declining creation.
    command.args(["env", "edit", target, "--editor", "/missing-editor"]);
    let mut terminal = Terminal::new(command);
    if target == "fixture/local" {
      terminal.reply("No secrets yet. Add some?", "\n");
    }
    assert_eq!(
      terminal.finish().success(),
      target == "fixture/local",
      "{}",
      terminal.transcript
    );
    assert!(!terminal.transcript.contains("Password:"));
    assert!(
      !terminal
        .transcript
        .contains("Open secrets with this custom editor command?")
    );
    assert!(!fixture.client_dir.join("secret-edit").exists());
  }
  let sensitive_actions: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM audit_events WHERE action IN ('secret.exported', 'admin.reauthenticated')"
  ).fetch_one(fixture.state.db.pool()).await.unwrap();
  assert_eq!(sensitive_actions, 0);
  fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn existing_sessions_confirm_plaintext_access_but_a_fresh_login_does_not_prompt_twice() {
  use super::super::support::EMAIL;
  let fixture = Fixture::new().await;
  let id = fixture.environment().await;
  fixture
    .request(
      Method::PUT,
      &format!("/api/v1/environments/{id}/secrets/OLD"),
      Some(json!({"value":"private-marker"})),
    )
    .await;
  for fresh_login in [false, true] {
    if fresh_login {
      app::cli::session::remove(&fixture.resolved()).unwrap();
    }
    let mut command = fixture.command();
    command.args(["env", "edit", &id, "--editor", "/usr/bin/true", "--dry-run"]);
    let mut terminal = Terminal::new(command);
    if fresh_login {
      terminal.reply("Email:", &format!("{EMAIL}\n"));
      terminal.reply("Password:", &format!("{PASSWORD}\n"));
    }
    terminal.reply("Open secrets with this custom editor command?", "y\n");
    if !fresh_login {
      terminal.reply("Password:", &format!("{PASSWORD}\n"));
    }
    assert!(terminal.finish().success(), "{}", terminal.transcript);
    assert!(!terminal.transcript.contains("private-marker"));
    assert!(!terminal.transcript.contains(PASSWORD));
    assert!(terminal.transcript.contains("Dry run complete."));
    assert_eq!(
      terminal
        .transcript
        .matches("Password confirmation required.")
        .count(),
      usize::from(!fresh_login)
    );
    let confirmations: i64 =
      sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='admin.reauthenticated'")
        .fetch_one(fixture.state.db.pool())
        .await
        .unwrap();
    assert_eq!(confirmations, 1);
  }
  fixture.stop().await;
}
