use clap::Parser;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "app=info,tower_http=info".into()),
        )
        .init();
    let cli = app::cli::Cli::parse();
    let json = cli.json;
    match app::cli::execute(cli).await {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({"success":false,"error":{"CLI_ERROR":error.to_string()}})
                );
            } else {
                eprintln!("Error: {error:#}");
            }
            std::process::exit(1)
        }
    }
}
