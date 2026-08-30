mod args;
mod client;
mod commands;
mod dotenv;
mod local_config;

pub use args::Cli;

pub async fn execute(cli: Cli) -> anyhow::Result<i32> {
  commands::execute(cli).await
}
