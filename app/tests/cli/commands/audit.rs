use super::support::{Fixture, output};
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn audit_verify_reports_an_intact_chain_and_flags_tampering() {
  let fixture = Fixture::new().await;
  fixture.environment().await;

  let human = fixture.run(&["audit", "verify"]).await;
  assert!(
    human.status.success(),
    "{}",
    String::from_utf8_lossy(&human.stderr)
  );
  let stdout = String::from_utf8_lossy(&human.stdout);
  assert!(stdout.contains("Chain"), "{stdout}");
  assert!(stdout.contains("intact"), "{stdout}");
  assert!(stdout.contains("Records verified"), "{stdout}");

  let report = fixture.json(&["audit", "verify"]).await;
  assert_eq!(report["ok"], json!(true));
  assert_eq!(report["unsequenced"], json!(0));
  assert_eq!(report["broken"], Value::Null);
  assert!(report["records"].as_u64().unwrap() >= 3);
  assert_eq!(report["headSeq"].as_u64(), report["records"].as_u64());
  // The human report prints the full head hash, which --anchor takes.
  assert!(
    stdout.contains(report["headHash"].as_str().unwrap()),
    "{stdout}"
  );

  // Editing a record in place makes verification fail with exit code 2, so
  // monitoring can alert on it. The guard trigger only stops application
  // bugs, so someone editing the file drops it first.
  sqlx::query("DROP TRIGGER audit_events_sealed_immutable")
    .execute(fixture.state.db.pool())
    .await
    .unwrap();
  sqlx::query("UPDATE audit_events SET action='tampered.for.test' WHERE seq=2")
    .execute(fixture.state.db.pool())
    .await
    .unwrap();

  let broken = fixture.run(&["audit", "verify"]).await;
  assert_eq!(broken.status.code(), Some(2));
  let stdout = String::from_utf8_lossy(&broken.stdout);
  assert!(stdout.contains("BROKEN at sequence 2"), "{stdout}");
  assert!(stdout.contains("Reason:"), "{stdout}");
  assert!(stdout.contains("Records verified"), "{stdout}");

  let mut command = fixture.command();
  command.args(["--json", "audit", "verify"]);
  let output = output(command, None).await;
  assert_eq!(output.status.code(), Some(2));
  let report: Value = serde_json::from_slice(&output.stdout).unwrap();
  assert_eq!(report["ok"], json!(false));
  assert_eq!(report["records"], json!(1));
  assert_eq!(report["broken"]["seq"], json!(2));
  assert!(
    report["broken"]["reason"]
      .as_str()
      .unwrap()
      .contains("hash"),
    "unexpected reason: {}",
    report["broken"]["reason"]
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_help_points_at_the_hash_chain() {
  let fixture = Fixture::new().await;
  let help = fixture.run(&["audit", "--help"]).await;
  assert!(help.status.success());
  let stdout = String::from_utf8_lossy(&help.stdout);
  assert!(stdout.contains("dopbase audit list"), "{stdout}");
  assert!(stdout.contains("dopbase audit verify"), "{stdout}");
  assert!(stdout.contains("hash chain"), "{stdout}");

  let help = fixture.run(&["audit", "list", "--help"]).await;
  assert!(help.status.success());
  let stdout = String::from_utf8_lossy(&help.stdout);
  for expected in ["--cursor", "--limit", "administrator", "No secret values"] {
    assert!(stdout.contains(expected), "{stdout}");
  }
}

/// Appends `count` chained audit records with one action and actor label.
async fn seed(
  fixture: &Fixture,
  action: &str,
  actor_label: &str,
  count: usize,
) {
  let mut transaction = fixture
    .state
    .db
    .pool()
    .begin_with("BEGIN IMMEDIATE")
    .await
    .unwrap();
  for index in 0..count {
    app::services::audit_chain::append(
      &mut transaction,
      &app::services::audit_chain::AuditEntry {
        id: &format!("aud_{action}_{index:04}"),
        actor_type: "admin",
        actor_id: Some("adm_1"),
        actor_label: Some(actor_label),
        action,
        project_id: None,
        environment_id: None,
        resource_type: None,
        resource_id: None,
        ip_address: None,
        metadata: "{}",
        created_at: &chrono::Utc::now().to_rfc3339(),
      },
    )
    .await
    .unwrap();
  }
  transaction.commit().await.unwrap();
}

fn json_lines(stdout: &[u8]) -> Vec<Value> {
  String::from_utf8_lossy(stdout)
    .lines()
    .filter(|line| !line.trim().is_empty())
    .map(|line| {
      serde_json::from_str(line).unwrap_or_else(|error| panic!("invalid JSON line {line}: {error}"))
    })
    .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_export_streams_json_lines_with_chain_fields_and_filters() {
  use super::support::{EMAIL, failure};
  let fixture = Fixture::new().await;
  fixture.environment().await;

  // Every line is one record as the list API returns it, including the
  // chain fields, so an export can be anchored and checked outside Dopbase.
  let human = fixture.run(&["audit", "export"]).await;
  assert!(
    human.status.success(),
    "{}",
    String::from_utf8_lossy(&human.stderr)
  );
  let records = json_lines(&human.stdout);
  assert!(records.len() >= 3, "exported {} records", records.len());
  let mut previous_seq = u64::MAX;
  for record in &records {
    let seq = record["seq"]
      .as_u64()
      .unwrap_or_else(|| panic!("missing seq: {record}"));
    assert!(record["action"].is_string(), "{record}");
    assert!(record["recordHash"].is_string(), "{record}");
    // Only the genesis record has no predecessor; every later record
    // carries the hash of the record before it.
    if seq == 1 {
      assert!(record["prevHash"].is_null(), "{record}");
    } else {
      assert!(record["prevHash"].is_string(), "{record}");
    }
    assert!(seq < previous_seq, "records are exported newest first");
    previous_seq = seq;
  }

  // Action, project and environment filters narrow the stream.
  let filtered = fixture
    .run(&["audit", "export", "--action", "environment.created"])
    .await;
  assert!(filtered.status.success());
  let records = json_lines(&filtered.stdout);
  assert!(!records.is_empty());
  assert!(
    records
      .iter()
      .all(|record| record["action"] == "environment.created"),
    "{records:?}"
  );

  let by_project = fixture
    .run(&["audit", "export", "--project", "fixture"])
    .await;
  assert!(by_project.status.success());
  let records = json_lines(&by_project.stdout);
  assert!(
    records
      .iter()
      .any(|record| record["action"] == "project.created")
  );
  assert!(records.iter().all(|record| record["projectId"].is_string()));

  let by_environment = fixture
    .run(&["audit", "export", "--environment", "fixture/local"])
    .await;
  assert!(by_environment.status.success());
  let records = json_lines(&by_environment.stdout);
  assert!(!records.is_empty());
  assert!(
    records
      .iter()
      .all(|record| record["environmentId"].is_string())
  );

  let by_actor = fixture.run(&["audit", "export", "--actor", EMAIL]).await;
  assert!(by_actor.status.success());
  assert!(!json_lines(&by_actor.stdout).is_empty());

  // Unknown references and a zero limit fail instead of exporting nothing.
  failure(
    &fixture
      .run(&["audit", "export", "--project", "absent"])
      .await,
    "Project not found",
  );
  failure(
    &fixture.run(&["audit", "export", "--limit", "0"]).await,
    "greater than zero",
  );

  // --limit caps the stream, and pages are followed beyond the first one.
  let limited = fixture.run(&["audit", "export", "--limit", "1"]).await;
  assert!(limited.status.success());
  assert_eq!(json_lines(&limited.stdout).len(), 1);

  // More records than one API page are followed through the pagination
  // cursor, so an export never silently stops at 200 records.
  seed(&fixture, "page.seeded", EMAIL, 205).await;
  let paged = fixture
    .run(&["audit", "export", "--action", "page.seeded"])
    .await;
  assert!(
    paged.status.success(),
    "{}",
    String::from_utf8_lossy(&paged.stderr)
  );
  assert_eq!(json_lines(&paged.stdout).len(), 205);
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_export_writes_a_private_file_and_needs_force_to_replace_it() {
  use super::support::failure;
  let fixture = Fixture::new().await;
  fixture.environment().await;
  let path = fixture.directory.path().join("audit.jsonl");

  let created = fixture
    .run(&["audit", "export", "--output", path.to_str().unwrap()])
    .await;
  assert!(
    created.status.success(),
    "{}",
    String::from_utf8_lossy(&created.stderr)
  );
  let contents = std::fs::read_to_string(&path).unwrap();
  let file_records = contents
    .lines()
    .filter(|line| !line.trim().is_empty())
    .map(|line| serde_json::from_str::<Value>(line).unwrap())
    .collect::<Vec<_>>();
  assert!(!file_records.is_empty());
  assert_eq!(
    file_records.len(),
    json_lines(&fixture.run(&["audit", "export"]).await.stdout).len()
  );
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
      std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
      0o600
    );
  }

  // An existing file is never replaced silently.
  failure(
    &fixture
      .run(&["audit", "export", "--output", path.to_str().unwrap()])
      .await,
    "already exists",
  );
  assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);

  // --force replaces it, and --json reports the count as an object.
  let replaced = fixture
    .json(&[
      "audit",
      "export",
      "--output",
      path.to_str().unwrap(),
      "--force",
    ])
    .await;
  assert_eq!(
    replaced["exported"].as_u64().unwrap(),
    file_records.len() as u64
  );
  assert_eq!(replaced["path"], path.to_str().unwrap());
  assert!(!std::fs::read_to_string(&path).unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_retention_archives_expired_records_and_verification_continues() {
  let fixture = Fixture::with_config(|config| config.audit_retention_days = Some(30)).await;
  fixture.environment().await;

  // The log went quiet past the retention window: every record written so
  // far is older than it.
  let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
  // Backdating sealed records needs the guard trigger out of the way.
  sqlx::query("DROP TRIGGER audit_events_sealed_immutable")
    .execute(fixture.state.db.pool())
    .await
    .unwrap();
  sqlx::query("UPDATE audit_events SET created_at='2000-01-01T00:00:00+00:00'")
    .execute(fixture.state.db.pool())
    .await
    .unwrap();

  // Audit writes never prune: retention runs at startup and on a timer.
  fixture.run(&["project", "create", "second"]).await;
  let after_write: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
  assert_eq!(after_write, before + 1);

  // A pass with the instance's own policy, as the timer runs it.
  let retention = fixture.state.config.audit_retention().unwrap();
  let removed = app::services::audit_chain::apply_retention(fixture.state.db.pool(), &retention)
    .await
    .unwrap();
  assert_eq!(removed as i64, before);

  // The expired records are gone from the log and archived as JSON lines
  // that keep every stored column, chain fields included.
  let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
    .fetch_one(fixture.state.db.pool())
    .await
    .unwrap();
  assert_eq!(
    remaining, 2,
    "only the record written after the backdating and the audit.pruned record survive"
  );
  let archive_dir = fixture.data_dir.join("audit-archive");
  let mut archives = std::fs::read_dir(&archive_dir)
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .collect::<Vec<_>>();
  archives.sort();
  assert_eq!(archives.len(), 1, "{archives:?}");
  let archived = std::fs::read_to_string(&archives[0]).unwrap();
  let archived_records = archived
    .lines()
    .map(|line| serde_json::from_str::<Value>(line).unwrap())
    .collect::<Vec<_>>();
  assert_eq!(archived_records.len() as i64, before);
  assert_eq!(archived_records[0]["seq"], json!(1));
  assert!(archived_records[0]["action"].is_string());
  assert!(archived_records[before as usize - 1]["recordHash"].is_string());

  // Verification continues from the anchor instead of reporting a gap.
  let human = fixture.run(&["audit", "verify"]).await;
  assert!(
    human.status.success(),
    "{}",
    String::from_utf8_lossy(&human.stderr)
  );
  let stdout = String::from_utf8_lossy(&human.stdout);
  assert!(stdout.contains("intact"), "{stdout}");
  assert!(stdout.contains("Pruned through"), "{stdout}");
  let report = fixture.json(&["audit", "verify"]).await;
  assert_eq!(report["ok"], json!(true));
  assert_eq!(report["records"], json!(2));
  assert_eq!(report["prunedSeq"], json!(before));
}

/// The arguments after `dopbase` in the next-page command `audit list`
/// prints to standard error, split by a real shell so quoting is checked
/// too, or `None` on the last page.
fn next_page(stderr: &[u8]) -> Option<Vec<String>> {
  let stderr = String::from_utf8_lossy(stderr);
  let (_, command) = stderr.split_once("Next page: dopbase ")?;
  let split = std::process::Command::new("sh")
    .arg("-c")
    .arg(format!("printf '%s\\n' {}", command.trim()))
    .output()
    .unwrap();
  assert!(split.status.success(), "{command}");
  Some(
    String::from_utf8(split.stdout)
      .unwrap()
      .lines()
      .map(str::to_owned)
      .collect(),
  )
}

fn ids(page: &Value) -> Vec<String> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["id"].as_str().unwrap().to_owned())
    .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_list_pages_newest_first_and_prints_the_next_page_command() {
  let fixture = Fixture::new().await;
  seed(&fixture, "page.seeded", "pager@example.com", 5).await;

  let first = fixture
    .run(&["audit", "list", "--action", "page.seeded", "--limit", "2"])
    .await;
  assert!(
    first.status.success(),
    "{}",
    String::from_utf8_lossy(&first.stderr)
  );
  let stdout = String::from_utf8_lossy(&first.stdout);
  for header in [
    "TIME",
    "SEQ",
    "ACTOR",
    "ACTION",
    "RESOURCE",
    "PROJECT",
    "ENVIRONMENT",
  ] {
    assert!(stdout.contains(header), "{stdout}");
  }
  assert!(stdout.contains("pager@example.com"), "{stdout}");
  assert!(stdout.contains("2 event(s)"), "{stdout}");
  // The hint goes to standard error so the table can be piped, and it
  // repeats the filters, because a cursor only makes sense with them.
  assert!(!stdout.contains("--cursor"), "{stdout}");
  let mut arguments = next_page(&first.stderr).expect("the first page links the next one");
  assert_eq!(
    &arguments[..6],
    ["audit", "list", "--action", "page.seeded", "--limit", "2"]
  );
  assert_eq!(arguments[6], "--cursor");

  // Following the printed commands walks every event once, newest first.
  let page = fixture
    .json(&["audit", "list", "--action", "page.seeded", "--limit", "2"])
    .await;
  assert_eq!(page["nextCursor"].as_str(), Some(arguments[7].as_str()));
  let mut seen = ids(&page);
  let mut pages = 1;
  loop {
    let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let page = fixture.json(&refs).await;
    let human = fixture.run(&refs).await;
    seen.extend(ids(&page));
    pages += 1;
    match next_page(&human.stderr) {
      Some(next) => arguments = next,
      None => {
        assert!(page["nextCursor"].is_null(), "{page}");
        break;
      }
    }
  }
  assert_eq!(pages, 3);
  let expected = (0..5)
    .rev()
    .map(|index| format!("aud_page.seeded_{index:04}"))
    .collect::<Vec<_>>();
  assert_eq!(seen, expected);

  // Filter values that need quoting survive the round trip.
  seed(&fixture, "it's odd", "pager@example.com", 2).await;
  let quoted = fixture
    .run(&["audit", "list", "--action", "it's odd", "--limit", "1"])
    .await;
  let arguments = next_page(&quoted.stderr).expect("two events, one per page");
  assert_eq!(&arguments[..4], ["audit", "list", "--action", "it's odd"]);
  let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
  assert_eq!(
    ids(&fixture.json(&refs).await),
    ["aud_it's odd_0000".to_owned()]
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_list_json_returns_the_api_page_unchanged() {
  let fixture = Fixture::new().await;
  fixture.environment().await;
  seed(&fixture, "page.seeded", "pager@example.com", 60).await;

  let api = fixture
    .request(reqwest::Method::GET, "/api/v1/audit-events", None)
    .await;
  let mut command = fixture.command();
  command.args(["--json", "audit", "list"]);
  let result = output(command, None).await;
  assert!(result.status.success());
  // Nothing besides the page is printed, not even the next-page hint.
  assert!(
    result.stderr.is_empty(),
    "{}",
    String::from_utf8_lossy(&result.stderr)
  );
  let page: Value = serde_json::from_slice(&result.stdout).unwrap();
  assert_eq!(page, api);
  assert_eq!(page["items"].as_array().unwrap().len(), 50);
  assert!(page["nextCursor"].is_string());
  assert!(page["items"][0]["recordHash"].is_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_list_filters_alone_and_combined() {
  use super::support::EMAIL;
  let fixture = Fixture::new().await;
  let environment_id = fixture.environment().await;
  let project_id = fixture.json(&["project", "show", "fixture"]).await["id"]
    .as_str()
    .unwrap()
    .to_owned();

  let all_match = |page: &Value, check: &dyn Fn(&Value) -> bool| {
    let items = page["items"].as_array().unwrap();
    assert!(!items.is_empty(), "{page}");
    assert!(items.iter().all(check), "{page}");
  };
  let page = fixture
    .json(&["audit", "list", "--action", "project.created"])
    .await;
  all_match(&page, &|item| item["action"] == "project.created");
  for reference in ["fixture", "FIXTURE", project_id.as_str()] {
    let page = fixture
      .json(&["audit", "list", "--project", reference])
      .await;
    all_match(&page, &|item| item["projectId"] == project_id.as_str());
  }
  for reference in ["fixture/local", environment_id.as_str()] {
    let page = fixture
      .json(&["audit", "list", "--environment", reference])
      .await;
    all_match(&page, &|item| {
      item["environmentId"] == environment_id.as_str()
    });
  }
  let page = fixture.json(&["audit", "list", "--actor", EMAIL]).await;
  all_match(&page, &|item| item["actorLabel"] == EMAIL);
  let page = fixture
    .json(&[
      "audit",
      "list",
      "--project",
      "fixture",
      "--action",
      "environment.created",
      "--actor",
      EMAIL,
    ])
    .await;
  all_match(&page, &|item| {
    item["action"] == "environment.created" && item["projectId"] == project_id.as_str()
  });

  // A filter that matches nothing is not an error.
  let empty = fixture
    .run(&["audit", "list", "--action", "nothing.matches"])
    .await;
  assert!(empty.status.success());
  assert!(String::from_utf8_lossy(&empty.stdout).contains("No audit events found."));
  assert!(next_page(&empty.stderr).is_none());
  let page = fixture
    .json(&["audit", "list", "--action", "nothing.matches"])
    .await;
  assert_eq!(page, json!({"items": [], "nextCursor": null}));
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_list_rejects_bad_input_before_or_from_the_server() {
  use super::support::failure;
  let fixture = Fixture::new().await;
  fixture.environment().await;
  for limit in ["0", "201", "ten"] {
    let result = fixture.run(&["audit", "list", "--limit", limit]).await;
    failure(&result, "--limit");
    assert!(result.stdout.is_empty());
  }
  failure(
    &fixture.run(&["audit", "list", "--project", "absent"]).await,
    "Project not found",
  );
  failure(
    &fixture
      .run(&["audit", "list", "--environment", "fixture/absent"])
      .await,
    "ENVIRONMENT_NOT_FOUND",
  );
  // The cursor is opaque to the CLI; the server rejects a bad one.
  failure(
    &fixture
      .run(&["audit", "list", "--cursor", "not a cursor"])
      .await,
    "The audit cursor is invalid.",
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_list_prints_stored_text_without_terminal_controls() {
  let fixture = Fixture::new().await;
  // Someone with write access to the database could plant escape codes
  // that erase the line or move the cursor to hide a record.
  let label = "evil\u{1b}[2K\rforged\u{202e}txt";
  seed(&fixture, "label.planted", label, 1).await;

  let human = fixture
    .run(&["audit", "list", "--action", "label.planted"])
    .await;
  assert!(human.status.success());
  let stdout = String::from_utf8_lossy(&human.stdout);
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(!stdout.contains('\r'), "{stdout:?}");
  assert!(!stdout.contains('\u{202e}'), "{stdout:?}");
  assert!(stdout.contains("evil?[2K?forged?txt"), "{stdout:?}");

  // JSON keeps the stored value exactly, escaped by the encoder.
  let page = fixture
    .json(&["audit", "list", "--action", "label.planted"])
    .await;
  assert_eq!(page["items"][0]["actorLabel"], label);
}

#[tokio::test(flavor = "multi_thread")]
async fn audit_list_shows_people_without_audit_read_only_their_own_actions() {
  use super::support::{browser_credentials, failure};
  let fixture = Fixture::new().await;
  fixture.environment().await;

  // A data token never reaches the audit log.
  let created = fixture.agent_token().await;
  let token = created["plaintextToken"].as_str().unwrap();
  let mut command = fixture.command();
  command.env("DOPBASE_TOKEN", token).args(["audit", "list"]);
  let result = output(command, None).await;
  failure(&result, "This token reads project data, not the audit log.");
  assert!(result.stdout.is_empty());
  assert!(!String::from_utf8_lossy(&result.stderr).contains(token));

  // A signed-in member sees their own actions and nothing else.
  let (cookie, csrf) = browser_credentials(&fixture).await;
  let api = reqwest::Client::new();
  let member: Value = api
    .post(format!("{}/api/v1/users", fixture.url))
    .header("cookie", &cookie)
    .header("x-dopbase-csrf", &csrf)
    .json(&json!({"email":"member@example.com","password":"correct-horse-456","role":"member"}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .await
    .unwrap();
  let login: Value = api
    .post(format!("{}/api/v1/auth/login", fixture.url))
    .json(&json!({"email":"member@example.com","password":"correct-horse-456","sessionKind":"cli"}))
    .send()
    .await
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .await
    .unwrap();
  app::cli::session::save(
    &fixture.resolved(),
    login["data"]["token"].as_str().unwrap(),
    Some("member@example.com"),
  )
  .unwrap();
  let events = fixture.json(&["audit", "list"]).await;
  let items = events["items"].as_array().unwrap();
  assert!(!items.is_empty(), "{events}");
  assert!(
    items
      .iter()
      .all(|event| event["actorId"] == member["data"]["id"]),
    "{events}"
  );
}
