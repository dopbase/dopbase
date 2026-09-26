use super::TokenCommand;
use crate::cli::{commands::environment, output};
use crate::{
  cli::{client, local_config},
  constants::api as api_paths,
};
use anyhow::Result;
use reqwest::Method;
use serde_json::json;

pub(crate) async fn execute(
  command: TokenCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  match command {
    TokenCommand::Create {
      environment,
      name,
      role,
      expires_in,
    } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::POST,
          &api_paths::tokens::collection(environment::env_id(&env)?),
          Some(json!({"name":name,"role":role,"expiresIn":expires_in})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        let token = data.get("token").unwrap_or(&serde_json::Value::Null);
        output::print_success(&format!("Created token {name} for {environment}."));
        output::print_fields(&[
          ("ID:", output::string(token, "id")),
          (
            "Expires:",
            token
              .get("expiresAt")
              .and_then(|value| value.as_str())
              .map_or_else(|| "never".into(), str::to_owned),
          ),
          ("Token:", output::string(&data, "plaintextToken")),
        ]);
        output::print_warning("Store this token now. Dopbase will not show it again.");
      }
    }
    TokenCommand::List { environment } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::GET,
          &api_paths::tokens::collection(environment::env_id(&env)?),
          None,
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        let rows = output::array(&data)
          .iter()
          .map(|token| {
            vec![
              output::string(token, "name"),
              output::string(token, "id"),
              if token.get("revokedAt").is_some_and(|value| !value.is_null()) {
                "revoked".into()
              } else if token
                .get("expiresAt")
                .and_then(|value| value.as_str())
                .is_some_and(|expiry| {
                  chrono::DateTime::parse_from_rfc3339(expiry)
                    .is_ok_and(|date| date <= chrono::Utc::now())
                })
              {
                "expired".into()
              } else {
                "active".into()
              },
              token
                .get("expiresAt")
                .and_then(|value| value.as_str())
                .map_or_else(|| "never".into(), str::to_owned),
              output::timestamp(token, "lastUsedAt"),
              output::timestamp(token, "createdAt"),
            ]
          })
          .collect::<Vec<_>>();
        output::print_table(
          &["NAME", "ID", "STATUS", "EXPIRES", "LAST USED", "CREATED"],
          &rows,
          &format!("No tokens found for {environment}."),
          &format!("{} token(s)", rows.len()),
        );
      }
    }
    TokenCommand::Revoke { token_id } => {
      let data = api
        .request(Method::POST, &api_paths::tokens::revoke(&token_id), None)
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Revoked token {token_id}."));
      }
    }
  }
  Ok(0)
}
