use app::cli::update::{UpdateStatus, is_newer, parse_release, parse_version, update_message};
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
fn compares_versions() {
  assert!(is_newer((0, 1, 0), (0, 0, 8)));
  assert!(is_newer((1, 0, 0), (0, 9, 9)));
  assert!(!is_newer((0, 0, 8), (0, 0, 8)));
  assert!(!is_newer((0, 0, 8), (0, 1, 0)));
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
