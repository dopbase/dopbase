use super::{environment, output};
use crate::cli::{args::TokenCommand, client, local_config};
use anyhow::Result;
use reqwest::Method;
use serde_json::json;

pub(super) async fn execute(
  command: TokenCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = client::human_client(server).await?;
  let data = match command {
    TokenCommand::Create {
      environment,
      name,
      role,
    } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::POST,
          &format!("/api/v1/environments/{}/tokens", environment::env_id(&env)?),
          Some(json!({"name":name,"role":role})),
        )
        .await?
    }
    TokenCommand::List { environment } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::GET,
          &format!("/api/v1/environments/{}/tokens", environment::env_id(&env)?),
          None,
        )
        .await?
    }
    TokenCommand::Revoke { token_id } => {
      api
        .request(
          Method::POST,
          &format!("/api/v1/tokens/{token_id}/revoke"),
          None,
        )
        .await?
    }
  };
  output::print_value(json_output, &data);
  Ok(0)
}
