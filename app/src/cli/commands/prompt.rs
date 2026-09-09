use anyhow::{Result, bail};
use std::io::{self, IsTerminal, Write};

pub(super) fn confirm(
  prompt: &str,
  yes: bool,
) -> Result<()> {
  if yes {
    return Ok(());
  }
  if !io::stdin().is_terminal() {
    bail!("confirmation is required; pass --yes for non-interactive use");
  }
  print!("{prompt} [y/N] ");
  io::stdout().flush()?;
  let mut answer = String::new();
  io::stdin().read_line(&mut answer)?;
  if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
    bail!("operation cancelled");
  }
  Ok(())
}
