use super::AccessArgs;
use crate::cli::{client, local_config, output};
use crate::constants::api as api_paths;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

/// `Write`, `Names only`, and so on, as the Admin UI says them.
fn level(value: &Value) -> String {
  match value.as_str().unwrap_or("-") {
    "none" => "no access".into(),
    "metadata" => "names only".into(),
    "project_manager" => "project manager".into(),
    other => other.into(),
  }
}

fn place(grant: &Value) -> String {
  match grant.get("scopeLabel").and_then(Value::as_str) {
    Some("") | None => "all projects".into(),
    Some(label) => label.into(),
  }
}

pub(crate) async fn execute(
  server: &local_config::ResolvedServer,
  args: AccessArgs,
  json_output: bool,
) -> Result<i32> {
  let api = client::metadata_client(server).await?;
  if let Some(project) = args.project {
    let data = api
      .request(Method::GET, &api_paths::projects::access(&project), None)
      .await?;
    if json_output {
      output::print_json(&data)?;
      return Ok(0);
    }
    let environments = output::array(&data["environments"])
      .iter()
      .map(|environment| {
        vec![
          output::string(environment, "name"),
          output::string(environment, "id"),
          if environment["protected"].as_bool() == Some(true) {
            "protected".into()
          } else {
            "-".into()
          },
        ]
      })
      .collect::<Vec<_>>();
    output::print_table(
      &["ENVIRONMENT", "ID", "PROTECTION"],
      &environments,
      &format!("{project} has no environments."),
      &format!("{} environment(s)", environments.len()),
    );
    let grants = output::array(&data["grants"])
      .iter()
      .map(|grant| {
        vec![
          output::string(grant, "principalLabel"),
          output::string(grant, "principalType"),
          place(grant),
          level(&grant["level"]),
        ]
      })
      .collect::<Vec<_>>();
    output::print_table(
      &["WHO", "KIND", "WHERE", "LEVEL"],
      &grants,
      &format!("Nobody has a grant on {project}."),
      &format!("{} grant(s)", grants.len()),
    );
    return Ok(0);
  }
  let user = args.user.unwrap_or_else(|| "me".into());
  let data = api
    .request(Method::GET, &api_paths::users::access(&user), None)
    .await?;
  if json_output {
    output::print_json(&data)?;
    return Ok(0);
  }
  let groups = output::array(&data["groups"])
    .iter()
    .map(|group| output::string(group, "name"))
    .collect::<Vec<_>>();
  let mut fields = vec![
    ("Account:", output::string(&data, "email")),
    (
      "Role:",
      format!(
        "{} (up to {})",
        output::string(&data, "roleName"),
        level(&data["maxLevel"])
      ),
    ),
  ];
  let owner = data
    .get("ownerEmail")
    .and_then(Value::as_str)
    .map(str::to_owned);
  if let Some(owner) = &owner {
    fields.push(("Owner:", owner.clone()));
  }
  fields.push((
    "Groups:",
    if groups.is_empty() {
      "none".into()
    } else {
      groups.join(", ")
    },
  ));
  if let Some(token) = data.get("token").filter(|token| !token.is_null()) {
    let scope = match token["scopeLabel"].as_str() {
      Some("") | None => "all projects".into(),
      Some(label) => label.to_owned(),
    };
    fields.push((
      "Token:",
      format!(
        "{}, up to {} on {scope}",
        output::string(token, "name"),
        level(&token["level"])
      ),
    ));
  }
  output::print_fields(&fields);
  let grants = output::array(&data["grants"]);
  let capped = grants
    .iter()
    .filter(|grant| grant.get("ownerLevel").is_some())
    .count();
  let rows = grants
    .iter()
    .map(|grant| {
      vec![
        place(grant),
        // An agent gets no more than its owner, so show what it really
        // gets and keep what was granted in brackets.
        match grant.get("ownerLevel") {
          Some(owner_level) => format!(
            "{} (granted {})",
            level(owner_level),
            level(&grant["level"])
          ),
          None => level(&grant["level"]),
        },
        if grant["principalType"] == "group" {
          format!("group {}", output::string(grant, "principalLabel"))
        } else {
          "direct".into()
        },
      ]
    })
    .collect::<Vec<_>>();
  let empty = match &owner {
    Some(owner) => format!("No grants. Its owner, {owner}, can give it access."),
    None => "No grants. Ask a project manager to share a project.".into(),
  };
  let mut summary = format!("{} grant(s)", rows.len());
  if capped > 0 {
    summary.push_str(&format!(
      "\nThe owner has less access on {capped} of them. An agent never gets more than its owner, so it gets the first level shown, not the one in brackets."
    ));
  }
  output::print_table(&["WHERE", "LEVEL", "FROM"], &rows, &empty, &summary);
  Ok(0)
}
