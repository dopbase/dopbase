use super::SecretCommand;
use crate::cli::{commands::environment, output, prompt};
use crate::{
  cli::{client, local_config},
  constants::api as api_paths,
  services::rotation,
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
  } else if matches!(
    &command,
    SecretCommand::List { .. } | SecretCommand::Get { reveal: false, .. }
  ) {
    client::metadata_client(server).await?
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
      let value = new_value(stdin, &key, "Secret value:")?;
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
        output::print_fields(&[("Version:", version(&data))]);
      }
    }
    SecretCommand::Get {
      environment,
      key,
      reveal,
      include_previous,
    } => {
      let env = environment::resolve_environment(&api, &environment).await?;
      let action = if reveal {
        api_paths::secrets::reveal(environment::env_id(&env)?, &key, include_previous)
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
      } else if include_previous {
        output::print_fields(&[
          ("Current:", output::string(&data, "value")),
          (
            "Previous:",
            data.get("previous").map_or_else(
              || "(none)".into(),
              |previous| output::string(previous, "value"),
            ),
          ),
          (
            "Previous valid until:",
            data.get("previous").map_or_else(
              || "never".into(),
              |previous| output::timestamp(previous, "expiresAt"),
            ),
          ),
        ]);
      } else if reveal {
        output::print_raw(&output::string(&data, "value"))?;
      } else {
        let mut fields = vec![
          ("Key:", output::string(&data, "key")),
          ("Version:", version(&data)),
          ("Created:", output::timestamp(&data, "createdAt")),
          ("Updated:", output::timestamp(&data, "updatedAt")),
        ];
        if data.get("previousExpiresAt").is_some() {
          fields.push((
            "Previous valid until:",
            output::timestamp(&data, "previousExpiresAt"),
          ));
        }
        output::print_fields(&fields);
      }
    }
    SecretCommand::Rotate {
      environment,
      key,
      grace,
      stdin,
    } => {
      let value = new_value(stdin, &key, "New secret value:")?;
      let grace_seconds = rotation::grace_duration(&grace)
        .map_err(anyhow::Error::msg)?
        .num_seconds();
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::POST,
          &api_paths::secrets::rotate(environment::env_id(&env)?, &key),
          Some(json!({"value":value,"graceSeconds":grace_seconds})),
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!("Rotated {key} in {environment}."));
        let mut fields = vec![("Version:", version(&data))];
        if data.get("previousExpiresAt").is_some() {
          fields.push((
            "Previous valid until:",
            output::timestamp(&data, "previousExpiresAt"),
          ));
        }
        output::print_fields(&fields);
      }
    }
    SecretCommand::Promote {
      environment,
      key,
      yes,
    } => {
      prompt::confirm(
        &format!("Promote the previous value of {key} in {environment}?"),
        yes,
      )?;
      let env = environment::resolve_environment(&api, &environment).await?;
      let data = api
        .request(
          Method::POST,
          &api_paths::secrets::promote(environment::env_id(&env)?, &key),
          None,
        )
        .await?;
      if json_output {
        output::print_json(&data)?;
      } else {
        output::print_success(&format!(
          "Promoted the previous value of {key} in {environment}."
        ));
        output::print_fields(&[
          ("Version:", version(&data)),
          (
            "Demoted value valid until:",
            output::timestamp(&data, "previousExpiresAt"),
          ),
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
/// Reads a new secret value from standard input or from a masked prompt that
/// displays * for each character.
fn new_value(
  stdin: bool,
  key: &str,
  label: &str,
) -> Result<String> {
  if stdin {
    return prompt::read_secret_stdin(key);
  }
  if !io::stdin().is_terminal() {
    bail!("use --stdin when providing a secret value non-interactively");
  }
  prompt::password(label, true, client::CliCancelled::SecretInput)
}
fn version(data: &Value) -> String {
  data
    .get("version")
    .and_then(Value::as_i64)
    .unwrap_or_default()
    .to_string()
}
