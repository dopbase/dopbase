use super::{
  args::{CacheAge, CacheCleanArgs, CacheCommand},
  store::{self, CacheMetadata},
};
use crate::cli::{client, local_config::ResolvedServer, output, prompt};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;

pub fn execute(
  command: CacheCommand,
  server: &ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  match command {
    CacheCommand::List => list(server, json_output),
    CacheCommand::Clean(args) => clean(server, args, json_output),
  }
}

fn list(
  server: &ResolvedServer,
  json_output: bool,
) -> Result<i32> {
  let token = cache_token(server)?;
  let snapshot = store::inspect(server, token.as_deref(), Utc::now())?;
  if json_output {
    output::print_json(&json!({
      "server_url": snapshot.server_url,
      "entry_count": snapshot.entries.len(),
      "entries": snapshot.entries,
    }))?;
  } else {
    let empty_message = format!(
      "No encrypted runtime cache entries found for {}.",
      snapshot.server_url
    );
    print_entries(
      &snapshot.server_url,
      &snapshot.entries,
      &empty_message,
      &cached_environment_count(snapshot.entries.len()),
    );
  }
  Ok(0)
}

fn clean(
  server: &ResolvedServer,
  args: CacheCleanArgs,
  json_output: bool,
) -> Result<i32> {
  let token = cache_token(server)?;
  let now = Utc::now();
  let snapshot = store::inspect(server, token.as_deref(), now)?;
  let age = args.older_than.unwrap_or_default();
  let matches = snapshot
    .entries
    .iter()
    .filter(|entry| args.all || entry.is_older_than(now, age.seconds()))
    .cloned()
    .collect::<Vec<_>>();

  if args.dry_run {
    print_clean_result(
      json_output,
      &snapshot.server_url,
      true,
      args.all,
      &age,
      &matches,
      matches.len(),
      0,
      snapshot.entries.len(),
    )?;
    return Ok(0);
  }

  if matches.is_empty() {
    print_clean_result(
      json_output,
      &snapshot.server_url,
      false,
      args.all,
      &age,
      &matches,
      0,
      0,
      snapshot.entries.len(),
    )?;
    return Ok(0);
  }

  prompt::confirm(&format!("Remove {}?", entry_count(matches.len())), args.yes)?;
  let token = token.as_deref().context(
    "Dopbase authentication is required to unlock the encrypted runtime cache. Run `dopbase login` first or set DOPBASE_TOKEN.",
  )?;
  let candidates = matches
    .iter()
    .map(CacheMetadata::candidate)
    .collect::<Vec<_>>();
  let removal = store::remove(server, token, &candidates, Utc::now())?;
  print_clean_result(
    json_output,
    &snapshot.server_url,
    false,
    args.all,
    &age,
    &removal.removed,
    matches.len(),
    removal.removed.len(),
    removal.remaining_count,
  )?;
  Ok(0)
}

fn cache_token(server: &ResolvedServer) -> Result<Option<String>> {
  if store::has_document(server)? {
    Ok(client::credential(server)?.token)
  } else {
    Ok(None)
  }
}

#[allow(clippy::too_many_arguments)]
fn print_clean_result(
  json_output: bool,
  server_url: &str,
  dry_run: bool,
  all: bool,
  age: &CacheAge,
  entries: &[CacheMetadata],
  matched_count: usize,
  removed_count: usize,
  remaining_count: usize,
) -> Result<()> {
  if json_output {
    output::print_json(&json!({
      "server_url": server_url,
      "dry_run": dry_run,
      "all": all,
      "older_than": if all { None } else { Some(age.to_string()) },
      "matched_count": matched_count,
      "removed_count": removed_count,
      "remaining_count": remaining_count,
      "entries": entries,
    }))?;
  } else if dry_run {
    print_entries(
      server_url,
      entries,
      "No encrypted runtime cache entries would be removed.",
      &format!("Dry run: {} would be removed.", entry_count(entries.len())),
    );
  } else if removed_count == 0 {
    output::print_text("No encrypted runtime cache entries were removed.");
  } else {
    print_entries(
      server_url,
      entries,
      "No encrypted runtime cache entries were removed.",
      &format!(
        "Removed {}. {}",
        entry_count(removed_count),
        remaining_count_label(remaining_count)
      ),
    );
  }
  Ok(())
}

fn cached_environment_count(count: usize) -> String {
  if count == 1 {
    "1 cached environment".into()
  } else {
    format!("{count} cached environments")
  }
}

fn remaining_count_label(count: usize) -> String {
  if count == 1 {
    "1 entry remains.".into()
  } else {
    format!("{count} entries remain.")
  }
}

fn entry_count(count: usize) -> String {
  if count == 1 {
    "1 encrypted runtime cache entry".into()
  } else {
    format!("{count} encrypted runtime cache entries")
  }
}

fn print_entries(
  server_url: &str,
  entries: &[CacheMetadata],
  empty_message: &str,
  summary: &str,
) {
  let rows = entries
    .iter()
    .map(|entry| {
      vec![
        server_url.to_owned(),
        entry.project.clone(),
        entry.environment.clone(),
        entry.environment_id.clone(),
        entry.fetched_at.clone(),
        entry.age.clone(),
        entry.aliases.join(", "),
      ]
    })
    .collect::<Vec<_>>();
  output::print_table(
    &[
      "SERVER",
      "PROJECT",
      "ENVIRONMENT",
      "ID",
      "FETCHED",
      "AGE",
      "ALIASES",
    ],
    &rows,
    empty_message,
    summary,
  );
}
