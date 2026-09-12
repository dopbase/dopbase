pub mod args;
#[doc(hidden)]
pub mod client;
#[doc(hidden)]
pub mod commands;
pub mod dotenv;
#[doc(hidden)]
pub mod environment_target;
#[doc(hidden)]
pub mod local_config;
#[doc(hidden)]
pub use commands::run::cache as runtime_cache;
#[doc(hidden)]
pub mod output;
#[doc(hidden)]
pub mod prompt;
pub mod secret_format;
#[doc(hidden)]
pub mod session;
pub use commands::update;

pub use args::Cli;
pub use client::CliCancelled;

pub async fn execute(cli: Cli) -> anyhow::Result<i32> {
  commands::execute(cli).await
}
