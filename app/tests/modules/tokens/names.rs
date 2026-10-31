use super::*;

#[tokio::test]
async fn runner_token_names_follow_the_environment_name_rule() {
  let (_directory, state, router, admin, environment_id) = admin_environment().await;
  let path = format!("/api/v1/environments/{environment_id}/tokens");
  let long = "a".repeat(64);
  for name in [
    "deploy prod",
    "Deploy",
    "2-deploy",
    "deploy_prod",
    "deploy--prod",
    "",
    long.as_str(),
  ] {
    let (status, body, _) = call(
      &router,
      "POST",
      &path,
      Some(&admin),
      Some(json!({"name": name, "role": "runner"})),
    )
    .await;
    assert_eq!(status, 422, "{name:?}: {body}");
    assert!(
      body["error"]["TOKEN_NAME_INVALID"].is_string(),
      "{name:?}: {body}"
    );
  }

  // Surrounding spaces are trimmed, as before.
  let (status, body, _) = call(
    &router,
    "POST",
    &path,
    Some(&admin),
    Some(json!({"name": " deploy-prod ", "role": "runner"})),
  )
  .await;
  assert_eq!(status, 201, "{body}");
  assert_eq!(body["data"]["token"]["name"], "deploy-prod");
  state.db.close().await;
}
