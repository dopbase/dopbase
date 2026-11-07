//! The provider presets compiled in from `presets.toml`.

use super::secrets_import::browser_login;
use super::sso_login::{
  Grant, MockIdp, callback, provider_body, save_provider, send, sso_fixture, start_login,
};
use super::*;
use app::modules::sso::presets;

#[test]
fn the_compiled_presets_keep_the_provider_quirks() {
  assert_eq!(presets::resolve("generic").id, presets::GENERIC_PRESET);
  // Google rejects `prompt=login`.
  assert!(!presets::resolve("google").prompt_login);
  assert!(presets::resolve("okta").prompt_login);
  // Entra marks no email as verified; only its domain ownership claim counts.
  let microsoft = presets::resolve("microsoft");
  assert_eq!(microsoft.email_verified_claim, "xms_edov");
  assert!(microsoft.rejects_issuer("https://login.microsoftonline.com/common/v2.0"));
  assert!(microsoft.rejects_issuer("https://login.microsoftonline.com/organizations"));
  assert!(!microsoft.rejects_issuer("https://login.microsoftonline.com/0f1e2d3c/v2.0"));
  assert_eq!(
    presets::resolve("keycloak").email_verified_claim,
    presets::STANDARD_EMAIL_VERIFIED_CLAIM
  );
  // An ID dropped from the file falls back to the generic preset.
  assert_eq!(presets::resolve("retired").id, presets::GENERIC_PRESET);
}

#[test]
fn a_broken_preset_file_is_refused() {
  let generic = r#"
[[preset]]
id = "generic"
label = "Other"
button_label = "SSO"
issuer = "https://idp.example.com"
hint = "Use the issuer."
"#;
  let extra = |fields: &str| {
    format!("{generic}\n[[preset]]\nlabel = \"x\"\nissuer = \"x\"\nhint = \"x\"\n{fields}\n")
  };
  assert_eq!(presets::parse(generic).unwrap().len(), 1);
  for (file, problem) in [
    ("preset = []".to_owned(), "\"generic\""),
    (format!("{generic}{generic}"), "twice"),
    (extra("id = \"Pocket ID\"\nbutton_label = \"x\""), "a-z"),
    (
      extra("id = \"pocket-id\"\nbutton_label = \" \""),
      "empty field",
    ),
    (
      extra("id = \"pocket-id\"\nbutton_label = \"x\"\nreject_issuer_segments = [\"/common/\"]"),
      "reject_issuer_message",
    ),
    // A misspelled key fails instead of being ignored.
    (
      extra("id = \"pocket-id\"\nbutton_label = \"x\"\npromptlogin = false"),
      "unknown field",
    ),
    (extra("id = \"pocket-id\""), "button_label"),
  ] {
    let error = presets::parse(&file).unwrap_err();
    assert!(error.contains(problem), "{problem}: {error}");
  }
}

#[tokio::test]
async fn settings_list_every_preset_and_refuse_unknown_ones() {
  let (_dir, state, router) = test_app().await;
  bootstrap_admin(&state, &router).await;
  let idp = MockIdp::start().await;
  let (cookie, csrf) = browser_login(&router).await;
  let (status, body, _) = send(
    &router,
    "GET",
    "/api/v1/sso/provider",
    &[&cookie],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  let listed: Vec<&str> = body["data"]["presets"]
    .as_array()
    .unwrap()
    .iter()
    .map(|preset| preset["id"].as_str().unwrap())
    .collect();
  let compiled: Vec<&str> = presets::all()
    .iter()
    .map(|preset| preset.id.as_str())
    .collect();
  assert_eq!(listed, compiled);
  // The UI gets labels and hints, not the server-side quirks.
  let microsoft = &body["data"]["presets"][1];
  assert_eq!(microsoft["buttonLabel"], "Microsoft");
  assert!(microsoft.get("emailVerifiedClaim").is_none());

  let mut request = provider_body(&idp);
  request["preset"] = json!("not-a-provider");
  let (status, response) = save_provider(&router, &cookie, &csrf, request).await;
  assert_eq!(status, 422);
  assert!(response["error"]["SSO_PRESET_INVALID"].is_string());
}

#[tokio::test]
async fn a_provider_whose_preset_was_removed_still_signs_in() {
  let fixture = sso_fixture().await;
  sqlx::query("UPDATE sso_providers SET preset='retired'")
    .execute(fixture.state.db.pool())
    .await
    .unwrap();
  let started = start_login(&fixture.router).await;
  fixture.idp.grant(
    "member-code",
    Grant {
      subject: "member-subject",
      email: "member@dopbase.com",
      email_verified: Some(true),
      nonce: started.nonce.clone(),
      auth_age: None,
    },
  );
  let handoff = callback(&fixture.router, &started, "member-code").await;
  assert!(handoff.starts_with("/login/sso#code="), "{handoff}");

  let (status, body, _) = send(
    &fixture.router,
    "GET",
    "/api/v1/sso/provider",
    &[&fixture.root_cookie],
    None,
    None,
  )
  .await;
  assert_eq!(status, 200, "{body}");
  assert_eq!(body["data"]["provider"]["preset"], "retired");
}
