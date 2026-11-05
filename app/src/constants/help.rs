pub(crate) const AFTER_HELP: &str = "\
Quickstart:
  dopbase server setup                     # initialize the local instance once
  dopbase server start                     # start on http://localhost:8840; fresh instances enter setup
  dopbase server start --background        # run the server in the background (also -b)
  dopbase server stop                      # stop the background server
  dopbase server restart                   # restart with saved launch settings
  dopbase login                            # authenticate with the active server
  dopbase init                             # create a project + environment from ./.env
  dopbase init myapp/dev --from .env       # create a project + environment from a secrets file
  dopbase secret set myapp/dev API_KEY     # set secret as need it, can be done via web ui
  dopbase run myapp/dev -- node server.js  # run with secrets injected as env vars

Common server options:
  --host <HOST>    Bind host (default: 127.0.0.1)
  --port <PORT>    Listen port (default: 8840)
  --no-web-ui      Disable the web UI for server start

Run 'dopbase help <command>' for details on any command.
";

/// A macro, not a const, so `concat!` can extend it for `run`.
macro_rules! environment_arg_help {
  () => {
    "Existing environment reference: an environment ID or \
PROJECT_REF/ENVIRONMENT_NAME. PROJECT_REF can be a project ID or name. For example: payment-service/production."
  };
}

pub(crate) const ENVIRONMENT_ARG_HELP: &str = environment_arg_help!();
pub(crate) const RUN_ENVIRONMENT_ARG_HELP: &str = concat!(
  environment_arg_help!(),
  " With a runner token, leave it out: the token's own environment is used, and a reference only checks it."
);
