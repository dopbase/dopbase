pub mod args;
#[doc(hidden)]
pub mod client;
#[doc(hidden)]
pub mod commands;
pub mod dotenv;
#[doc(hidden)]
pub mod local_config;
#[doc(hidden)]
pub mod runtime_cache;
#[doc(hidden)]
pub mod session;
pub mod update;

pub use args::Cli;
pub use client::CliCancelled;

pub async fn execute(cli: Cli) -> anyhow::Result<i32> {
  commands::execute(cli).await
}
