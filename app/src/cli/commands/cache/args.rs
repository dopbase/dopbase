use clap::{Args, Subcommand};
use std::{fmt, str::FromStr};

pub const DEFAULT_MAX_AGE_SECONDS: i64 = 14 * 24 * 60 * 60;

pub const HELP: &str = "Examples:
  dopbase cache list
  dopbase cache clean
  dopbase cache clean --older-than 30d
  dopbase cache clean --all --yes";

pub const LIST_HELP: &str = "Examples:
  dopbase cache list
  dopbase cache list --json";

pub const CLEAN_HELP: &str = "Examples:
  dopbase cache clean --dry-run
  dopbase cache clean --older-than 30d --yes
  dopbase cache clean --all --yes";

#[derive(Debug, Subcommand)]
pub enum CacheCommand {
  /// Show cached environments for the active server.
  #[command(after_help = LIST_HELP)]
  List,
  /// Remove cached environments by age, or remove every entry with --all.
  #[command(after_help = CLEAN_HELP)]
  Clean(CacheCleanArgs),
}

#[derive(Args, Debug)]
pub struct CacheCleanArgs {
  /// Remove entries older than this age. Units: s, m, h, or d. Defaults to 14d.
  #[arg(long, value_name = "AGE", conflicts_with = "all")]
  pub older_than: Option<CacheAge>,
  /// Remove every cached environment, regardless of age.
  #[arg(long)]
  pub all: bool,
  /// Show what would be removed without changing the cache.
  #[arg(long)]
  pub dry_run: bool,
  /// Skip the confirmation prompt.
  #[arg(long)]
  pub yes: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheAge {
  seconds: i64,
  display: String,
}

impl CacheAge {
  pub fn seconds(&self) -> i64 {
    self.seconds
  }
}

impl Default for CacheAge {
  fn default() -> Self {
    Self {
      seconds: DEFAULT_MAX_AGE_SECONDS,
      display: "14d".into(),
    }
  }
}

impl fmt::Display for CacheAge {
  fn fmt(
    &self,
    formatter: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    formatter.write_str(&self.display)
  }
}

impl FromStr for CacheAge {
  type Err = String;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    let (amount, multiplier) = [("s", 1), ("m", 60), ("h", 60 * 60), ("d", 24 * 60 * 60)]
      .into_iter()
      .find_map(|(unit, multiplier)| value.strip_suffix(unit).map(|amount| (amount, multiplier)))
      .ok_or_else(|| "use a positive whole number followed by s, m, h, or d".to_owned())?;
    let amount = amount
      .parse::<i64>()
      .map_err(|_| "use a positive whole number followed by s, m, h, or d".to_owned())?;
    if amount <= 0 {
      return Err("cache age must be greater than zero".into());
    }
    let seconds = amount
      .checked_mul(multiplier)
      .ok_or_else(|| "cache age is too large".to_owned())?;
    Ok(Self {
      seconds,
      display: value.to_owned(),
    })
  }
}
