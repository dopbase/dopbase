use crate::cli::{
  commands::environment,
  output, prompt,
};
use crate::{
  cli::{args::SecretCommand, client, local_config},
  constants::api as api_paths,
};
use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::{Value, json};
use std::io::{self, IsTerminal};

pub(crate) async fn execute(
  command: SecretCommand,
  server: &local_config::ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let api = if matches!(&command, SecretCommand::Get { reveal: true, .. }) {
    client::recently_authenticated_client(server).await?
  } else {
    client::human_client(server).await?
  };
  match command {
    SecretCommand::List { environment } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::GET,
          &api_paths::secrets::collection(environment::env_id(&env)?),
          None,
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        let rows = output::array(&data)
          .iter()
          .map(|secret| {
            vec![
              output::string(secret, "key"),
              secret
                .get("version")
                .and_then(Value::as_i64)
                .unwrap_or_default()
                .to_string(),
              output::timestamp(secret, "updatedAt"),
            ]
          })
          .collect::<Vec<_>>();
        output::print_table(
          &["KEY", "VERSION", "UPDATED"],
          &rows,
          &format!("No secrets found in {environment}."),
          &format!("{} secret(s)", rows.len()),
        );
      }
    }
    SecretCommand::Set {
      environment,
      key,
      stdin,
    } => {
      let value = if stdin {
        prompt::read_secret_stdin(&key)?
      } else {
        if !io::stdin().is_terminal() {
          bail!("use --stdin when setting a secret non-interactively");
        }
        prompt::password("Secret value:", true, client::CliCancelled::SecretInput)?
      };
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::PUT,
          &api_paths::secrets::item(environment::env_id(&env)?, &key),
          Some(json!({"value":value})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Saved {key} in {environment}."));
        output::print_fields(&[(
          "Version:",
          data
            .get("version")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            .to_string(),
        )]);
      }
    }
    SecretCommand::Get {
      environment,
      key,
      reveal,
    } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      let action = if reveal {
        api_paths::secrets::reveal(environment::env_id(&env)?, &key)
      } else {
        api_paths::secrets::item(environment::env_id(&env)?, &key)
      };
      let data = api
        .request(
          if reveal { Method::POST } else { Method::GET },
          &action,
          None,
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else if reveal {
        output::print_raw(&output::string(&data, "value"))?;
      } else {
        output::print_fields(&[
          ("Key:", output::string(&data, "key")),
          (
            "Version:",
            data
              .get("version")
              .and_then(Value::as_i64)
              .unwrap_or_default()
              .to_string(),
          ),
          ("Created:", output::timestamp(&data, "createdAt")),
          ("Updated:", output::timestamp(&data, "updatedAt")),
        ]);
      }
    }
    SecretCommand::Delete {
      environment,
      key,
      yes,
    } => {
      prompt::confirm(&format!("Delete secret {key} from {environment}?"), yes)?;
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::DELETE,
          &api_paths::secrets::item(environment::env_id(&env)?, &key),
          None,
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Deleted {key} from {environment}."));
      }
    }
  }
  Ok(0)
}
