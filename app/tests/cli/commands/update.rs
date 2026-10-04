use app::cli::update::{UpdateStatus, parse_release, parse_version, update_message};
use serde_json::{Value, json};

#[test]
fn parses_version_triplets() {
  assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
  assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
  assert_eq!(parse_version(" v0.0.12 "), Some((0, 0, 12)));
  assert_eq!(parse_version("1.2"), None);
  assert_eq!(parse_version("1.2.3.4"), None);
  assert_eq!(parse_version("1.2.3-rc1"), None);
  assert_eq!(parse_version("a.b.c"), None);
  assert_eq!(parse_version(""), None);
}

#[test]
fn parses_release_payload() {
  let release = parse_release(&json!({
      "tag_name": "0.1.0",
      "html_url": "https://github.com/dopbase/dopbase/releases/tag/0.1.0"
  }))
  .unwrap();
  assert_eq!(release.tag, "0.1.0");
  assert_eq!(release.version, Some((0, 1, 0)));
  assert_eq!(
    release.url,
    "https://github.com/dopbase/dopbase/releases/tag/0.1.0"
  );
}

#[test]
fn rejects_release_without_tag() {
  assert!(parse_release(&json!({ "html_url": "https://example.com" })).is_err());
  assert!(parse_release(&json!({ "tag_name": "release-1" })).is_err());
  assert!(parse_release(&Value::Null).is_err());
}

#[test]
fn update_status_serializes_the_documented_shape() {
  let status = UpdateStatus {
    current_version: "0.0.12",
    latest_version: "0.1.0".into(),
    update_available: true,
    release_url: "https://github.com/dopbase/dopbase/releases/tag/0.1.0".into(),
  };
  let value = serde_json::to_value(&status).unwrap();
  assert_eq!(
    value,
    json!({
        "current_version": "0.0.12",
        "latest_version": "0.1.0",
        "update_available": true,
        "release_url": "https://github.com/dopbase/dopbase/releases/tag/0.1.0"
    })
  );
}

#[test]
fn update_message_explains_how_to_install_safely() {
  let status = UpdateStatus {
    current_version: "0.0.12",
    latest_version: "0.1.0".into(),
    update_available: true,
    release_url: "https://github.com/dopbase/dopbase/releases/tag/0.1.0".into(),
  };

  let message = update_message(&status);

  assert!(message.contains("A new Dopbase release is available"));
  assert!(message.contains("Current version  \u{1b}[36m0.0.12"));
  assert!(message.contains("Latest version   \u{1b}[36m0.1.0"));
  assert!(message.contains("Stop every running Dopbase server before updating."));
  assert!(message.contains("curl -fsSL https://dopbase.com/install.sh | sh"));
  assert!(message.contains("https://github.com/dopbase/dopbase/releases/tag/0.1.0"));
}

#[tokio::test]
async fn release_check_uses_the_http_contract_and_reports_availability() {
  use app::cli::commands::update::check_release;
  use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    routing::get,
  };
  for (tag, status, body, expected) in [
    (
      "1.3.0",
      200,
      r#"{"tag_name":"1.3.0","html_url":"https://example.com/release"}"#,
      Some(true),
    ),
    (
      "1.2.3",
      200,
      r#"{"tag_name":"1.2.3","html_url":"https://example.com/release"}"#,
      Some(false),
    ),
    (
      "1.1.0",
      200,
      r#"{"tag_name":"1.1.0","html_url":"https://example.com/release"}"#,
      Some(false),
    ),
    ("", 404, "{}", None),
    ("", 403, "{}", None),
    ("", 200, "invalid-json", None),
  ] {
    let router = Router::new().route(
      "/latest",
      get(move |headers: HeaderMap| async move {
        assert_eq!(headers["user-agent"], "dopbase/1.2.3");
        assert_eq!(headers["accept"], "application/vnd.github+json");
        assert_eq!(headers["x-github-api-version"], "2022-11-28");
        (
          StatusCode::from_u16(status).unwrap(),
          [("content-type", "application/json")],
          body,
        )
      }),
    );
    let (url, task) = super::support::serve(router).await;
    let result = check_release("1.2.3", &format!("{url}/latest")).await;
    task.abort();
    if let Some(available) = expected {
      let result = result.unwrap();
      assert_eq!(result.update_available, available);
      assert_eq!(result.latest_version, tag);
      assert_eq!(result.release_url, "https://example.com/release");
    } else {
      let error = result.unwrap_err().to_string();
      assert!(error.contains(match status {
        404 => "no published",
        403 => "request was rejected",
        _ => "failed to decode",
      }));
    }
  }
}
