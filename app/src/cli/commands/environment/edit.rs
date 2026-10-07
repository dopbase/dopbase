use crate::{
  cli::{
    client::{self, ApiClient, CliCancelled},
    commands::environment,
    dotenv, local_config, output, prompt,
    secret_editor::{Editor, Session},
  },
  constants::api,
  models::SecretInput,
};
use anyhow::{Context, Result, bail};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal};
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
  entries: Vec<SecretInput>,
  revision: String,
  env_layout: Option<String>,
}
impl Drop for Snapshot {
  fn drop(&mut self) {
    for entry in &mut self.entries {
      entry.value.zeroize();
    }
  }
}

struct Draft {
  entries: Vec<SecretInput>,
  layout: String,
}
impl Drop for Draft {
  fn drop(&mut self) {
    for entry in &mut self.entries {
      entry.value.zeroize();
    }
    self.layout.zeroize();
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Import<'a> {
  mode: &'static str,
  dry_run: bool,
  entries: &'a [SecretInput],
  env_layout: &'a str,
  expected_revision: &'a str,
}

pub(super) async fn execute(
  server: &local_config::ResolvedServer,
  reference: &str,
  explicit_editor: Option<&str>,
  dry_run: bool,
  json_output: bool,
) -> Result<i32> {
  if json_output {
    bail!("env edit cannot be combined with --json");
  }
  if !cfg!(unix) {
    bail!("env edit currently supports Linux and macOS");
  }
  if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
    bail!("env edit requires an interactive terminal");
  }
  let authentication = client::acquire_human_client(server).await?;
  let api = authentication.client();
  let env = environment::resolve_environment(api, reference).await?;
  let id = environment::env_id(&env)?;
  let metadata = api
    .request(Method::GET, &api::secrets::collection(id), None)
    .await?;
  if output::array(&metadata).is_empty()
    && !prompt::choice(
      "No secrets yet. Add some?",
      false,
      CliCancelled::SecretInput,
    )?
  {
    output::print_text("No changes to save.");
    return Ok(0);
  }
  let editor = Editor::resolve(explicit_editor)?;
  if !editor.protected() {
    output::print_warning(
      "This editor command uses its own configuration. Plugins, backups, history, and external services may retain plaintext secrets. The editor must stay open until editing finishes.",
    );
    prompt::confirm("Open secrets with this custom editor command?", false)?;
  }
  crate::cli::secret_editor::disable_core_dumps()?;
  let api = authentication.confirm_plaintext_access().await?;
  output::print_fields(&[
    ("Server:", server.url.clone()),
    ("Environment:", reference.into()),
    ("Environment ID:", id.into()),
  ]);
  output::print_warning(
    "Comments are unencrypted metadata. Keep credentials in values, never in comments.",
  );
  let snapshot: Snapshot = serde_json::from_value(
    api
      .request(Method::POST, &api::secrets::export(id), None)
      .await?,
  )
  .map_err(|_| {
    anyhow::anyhow!("server does not support editor snapshots; upgrade the server before editing")
  })?;
  if snapshot.revision.is_empty() {
    bail!("server returned an invalid editor revision");
  }
  let text = Zeroizing::new(dotenv::render_layout(
    snapshot.env_layout.as_deref(),
    &snapshot.entries,
  )?);
  let root = server
    .config_path
    .parent()
    .context("client configuration has no directory")?
    .join("secret-edit");
  #[cfg(unix)]
  let mut termination = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
  let mut session = Session::new(&root)?;
  let written = session.write(&text);
  drop(text);
  if let Err(error) = written {
    session.close().await?;
    return Err(error);
  }
  let preparation = prepare(
    &api,
    id,
    reference,
    &snapshot,
    &editor,
    &mut session,
    dry_run,
  );
  #[cfg(unix)]
  let result = tokio::select! {
    result = preparation => result,
    signal = tokio::signal::ctrl_c() => {
      signal?;
      Err(CliCancelled::SecretInput.into())
    },
    _ = termination.recv() => Err(CliCancelled::SecretInput.into()),
  };
  #[cfg(not(unix))]
  let result = preparation.await;
  // No editor files are needed once the user has confirmed the draft.
  session.close().await?;
  let Some(draft) = result? else { return Ok(0) };
  let request = Import {
    mode: "replace",
    dry_run: false,
    entries: &draft.entries,
    env_layout: &draft.layout,
    expected_revision: &snapshot.revision,
  };
  let import_path = api::secrets::import(id);
  output::print_text("Saving...");
  let save = api.request_json(Method::POST, &import_path, &request);
  tokio::pin!(save);
  #[cfg(unix)]
  tokio::select! {
    result = &mut save => { result?; },
    signal = tokio::signal::ctrl_c() => {
      signal?;
      output::print_text("Save already requested. Waiting for the server response.");
      save.await?;
    },
    _ = termination.recv() => {
      output::print_text("Save already requested. Waiting for the server response.");
      save.await?;
    },
  }
  #[cfg(not(unix))]
  save.await?;
  output::print_success(&format!("Saved secrets in {reference}."));
  Ok(0)
}

async fn prepare(
  api: &ApiClient,
  id: &str,
  reference: &str,
  snapshot: &Snapshot,
  editor: &Editor,
  session: &mut Session,
  dry_run: bool,
) -> Result<Option<Draft>> {
  loop {
    session.run(editor).await?;
    let text = session.read()?;
    let draft = match dotenv::parse_document(&text) {
      Ok((entries, layout)) => Draft { entries, layout },
      Err(error) => {
        output::print_warning(&error.to_string());
        if ask(|| prompt::choice("Edit again?", false, CliCancelled::SecretInput)).await? {
          continue;
        }
        return Err(CliCancelled::SecretInput.into());
      }
    };
    drop(text);
    let request = Import {
      mode: "replace",
      dry_run: true,
      entries: &draft.entries,
      env_layout: &draft.layout,
      expected_revision: &snapshot.revision,
    };
    output::print_text("Checking changes...");
    let preview = api
      .request_json(Method::POST, &api::secrets::import(id), &request)
      .await?;
    preview
      .get("revision")
      .and_then(serde_json::Value::as_str)
      .filter(|revision| *revision == snapshot.revision)
      .context("server returned an inconsistent editor preview")?;
    let layout_changed = snapshot.env_layout.as_deref().unwrap_or_default() != draft.layout;
    output::print_fields(&[(
      "Layout:",
      if layout_changed {
        "changed"
      } else {
        "unchanged"
      }
      .into(),
    )]);
    let mut changed = layout_changed;
    for (label, field) in [
      ("Added:", "addedKeys"),
      ("Updated:", "updatedKeys"),
      ("Deleted:", "deletedKeys"),
    ] {
      let keys = preview
        .get(field)
        .and_then(serde_json::Value::as_array)
        .context("server returned an invalid editor preview")?;
      changed |= !keys.is_empty();
      let names = keys
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
      let description = if keys.is_empty() {
        "0".into()
      } else {
        format!("{} ({names})", keys.len())
      };
      output::print_fields(&[(label, description)]);
    }
    if dry_run {
      output::print_text("Dry run complete. No changes were saved.");
      return Ok(None);
    }
    if !changed {
      output::print_text("No changes to save.");
      return Ok(None);
    }
    let action = ask(|| {
      prompt::text(
        "(a)Apply, (e)edit again, or (c)cancel? [cancel]:",
        CliCancelled::SecretInput,
      )
    })
    .await?;
    match action.trim().to_ascii_lowercase().as_str() {
      "edit" | "e" => continue,
      "apply" | "a" | "y" => {}
      _ => return Err(CliCancelled::SecretInput.into()),
    }
    if draft.entries.is_empty() && !snapshot.entries.is_empty() {
      let question = format!("Type {reference} to delete every secret:");
      let confirmation = ask(move || prompt::text(&question, CliCancelled::SecretInput)).await?;
      if confirmation != reference {
        return Err(CliCancelled::SecretInput.into());
      }
    }
    return Ok(Some(draft));
  }
}

// Independent prompt workers let signal handling continue while inquire waits
// for input. Session cleanup restores the terminal if a signal ends a prompt.
async fn ask<T: Send + 'static>(
  question: impl FnOnce() -> Result<T> + Send + 'static
) -> Result<T> {
  let (sender, receiver) = tokio::sync::oneshot::channel();
  std::thread::Builder::new()
    .name("env-edit-prompt".into())
    .spawn(move || {
      let _ = sender.send(question());
    })?;
  receiver.await.context("editor prompt ended unexpectedly")?
}
