#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_writer(std::io::stderr)
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "app=info,tower_http=info".into()),
    )
    .init();
  let cli = app::cli::Cli::parse_with_help();
  let json = cli.json;
  match app::cli::execute(cli).await {
    Ok(code) => std::process::exit(code),
    Err(error) => {
      if let Some(cancelled) = error.downcast_ref::<app::cli::CliCancelled>() {
        if json {
          eprintln!(
            "{}",
            serde_json::json!({"success":false,"error":{"CLI_CANCELLED":cancelled.to_string()}})
          );
        } else {
          eprintln!("{cancelled}");
        }
        std::process::exit(130)
      } else if let Some(required) = error.downcast_ref::<app::server::InitializationRequired>() {
        if json {
          println!(
            "{}",
            serde_json::json!({
              "success": false,
              "info": {
                "code": "SETUP_REQUIRED",
                "message": required.to_string(),
                "data_dir": required.data_dir,
                "config_file": required.config_path,
              }
            })
          );
        } else {
          app::cli::output::print_info(&required.to_string());
        }
        std::process::exit(1)
      } else if json {
        if let Some(initialized) =
          error.downcast_ref::<app::cli::commands::server::SetupCommittedError>()
        {
          eprintln!(
            "{}",
            serde_json::json!({"success":false,"initialized":true,"email":initialized.email,"data_dir":initialized.data_dir,"error":{"CLI_ERROR":initialized.to_string()}})
          );
          std::process::exit(1);
        }
        eprintln!(
          "{}",
          serde_json::json!({"success":false,"error":{"CLI_ERROR":error.to_string()}})
        );
        std::process::exit(1)
      } else {
        let style = anstyle::Style::new()
          .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)))
          .effects(anstyle::Effects::BOLD);
        anstream::eprintln!("{style}Error:{style:#} {error:#}");
        std::process::exit(1)
      }
    }
  }
}
