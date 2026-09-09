use super::{environment, output, prompt};
use crate::cli::{args::SecretCommand, client, local_config};
use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::json;
use std::io::{self, IsTerminal, Read};

pub(super) async fn execute(
  command: SecretCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = if matches!(&command, SecretCommand::Get { reveal: true, .. }) {
    client::recently_authenticated_client(server).await?
  } else {
    client::human_client(server).await?
  };
  let data = match command {
    SecretCommand::List { environment } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::GET,
          &format!(
            "/api/v1/environments/{}/secrets",
            environment::env_id(&env)?
          ),
          None,
        )
        .await?
    }
    SecretCommand::Set {
      environment,
      key,
      stdin,
    } => {
      let value = if stdin {
        let mut value = String::new();
        io::stdin().read_to_string(&mut value)?;
        value
      } else {
        if !io::stdin().is_terminal() {
          bail!("use --stdin when setting a secret non-interactively");
        }
        rpassword::prompt_password("Secret value: ")?
      };
      let env = environment::resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::PUT,
          &format!(
            "/api/v1/environments/{}/secrets/{key}",
            environment::env_id(&env)?
          ),
          Some(json!({"value":value})),
        )
        .await?
    }
    SecretCommand::Get {
      environment,
      key,
      reveal,
    } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      let action = if reveal {
        format!(
          "/api/v1/environments/{}/secrets/{key}/reveal",
          environment::env_id(&env)?
        )
      } else {
        format!(
          "/api/v1/environments/{}/secrets/{key}",
          environment::env_id(&env)?
        )
      };
      api
        .request(
          if reveal { Method::POST } else { Method::GET },
          &action,
          None,
        )
        .await?
    }
    SecretCommand::Delete {
      environment,
      key,
      yes,
    } => {
      prompt::confirm(&format!("Delete secret {key} from {environment}?"), yes)?;
      let env = environment::resolve_environment(&api, &environment).await?;
      api
        .request(
          Method::DELETE,
          &format!(
            "/api/v1/environments/{}/secrets/{key}",
            environment::env_id(&env)?
          ),
          None,
        )
        .await?
    }
  };
  output::print_value(json_output, &data);
  Ok(0)
}
