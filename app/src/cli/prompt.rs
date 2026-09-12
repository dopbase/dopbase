use crate::cli::client::CliCancelled;
use anyhow::{Result, bail};
use inquire::{Confirm, InquireError, Password, PasswordDisplayMode, Text};
use std::io::{self, IsTerminal, Read};

fn map_error(
  error: InquireError,
  cancelled: CliCancelled,
) -> anyhow::Error {
  match error {
    InquireError::OperationCanceled | InquireError::OperationInterrupted => cancelled.into(),
    error => anyhow::anyhow!(error),
  }
}

pub(crate) fn text(
  label: &str,
  cancelled: CliCancelled,
) -> Result<String> {
  Text::new(label)
    .prompt()
    .map_err(|error| map_error(error, cancelled))
}

pub(crate) fn email(
  label: &str,
  cancelled: CliCancelled,
) -> Result<String> {
  Text::new(label)
    .with_validator(
      |value: &str| match crate::modules::common::validate_email(value) {
        Ok(_) => Ok(inquire::validator::Validation::Valid),
        Err(_) => Ok(inquire::validator::Validation::Invalid(
          "Enter a valid email address.".into(),
        )),
      },
    )
    .prompt()
    .map_err(|error| map_error(error, cancelled))
    .and_then(|value| {
      crate::modules::common::validate_email(&value)
        .map_err(|_| anyhow::anyhow!("Enter a valid email address."))
    })
}

pub(crate) fn password(
  label: &str,
  allow_empty: bool,
  cancelled: CliCancelled,
) -> Result<String> {
  Password::new(label)
    .with_display_mode(PasswordDisplayMode::Masked)
    .without_confirmation()
    .with_validator(move |value: &str| {
      if allow_empty || !value.is_empty() {
        Ok(inquire::validator::Validation::Valid)
      } else {
        Ok(inquire::validator::Validation::Invalid(
          "Enter a password.".into(),
        ))
      }
    })
    .prompt()
    .map_err(|error| map_error(error, cancelled))
}

pub(crate) fn new_password(
  label: &str,
  confirmation_label: &str,
) -> Result<String> {
  Password::new(label)
    .with_display_mode(PasswordDisplayMode::Masked)
    .with_custom_confirmation_message(confirmation_label)
    .with_custom_confirmation_error_message("Passwords do not match.")
    .with_validator(
      |value: &str| match crate::modules::common::validate_password(value) {
        Ok(()) => Ok(inquire::validator::Validation::Valid),
        Err(error) => Ok(inquire::validator::Validation::Invalid(
          error
            .errors
            .into_values()
            .collect::<Vec<_>>()
            .join(" ")
            .into(),
        )),
      },
    )
    .prompt()
    .map_err(|error| map_error(error, CliCancelled::PasswordConfirmation))
}

pub(super) fn confirm(
  question: &str,
  yes: bool,
) -> Result<()> {
  confirm_with_cancel(question, yes, CliCancelled::Confirmation)
}

pub(crate) fn confirm_with_cancel(
  question: &str,
  yes: bool,
  cancelled: CliCancelled,
) -> Result<()> {
  if yes {
    return Ok(());
  }
  if !io::stdin().is_terminal() {
    bail!("confirmation is required. Pass --yes for non-interactive use");
  }
  let confirmed = Confirm::new(question)
    .with_default(false)
    .prompt()
    .map_err(|error| map_error(error, cancelled))?;
  if !confirmed {
    return Err(cancelled.into());
  }
  Ok(())
}

pub(crate) fn read_secret_stdin(key: &str) -> Result<String> {
  let interactive = io::stdin().is_terminal();
  if interactive {
    eprintln!("Paste the secret value below.");
    #[cfg(windows)]
    eprintln!("Press Ctrl+Z, then Enter when finished.");
    #[cfg(not(windows))]
    eprintln!("Press Ctrl+D when finished.");
  }

  let mut value = String::new();
  io::stdin().read_to_string(&mut value).map_err(|error| {
    if error.kind() == io::ErrorKind::Interrupted {
      anyhow::Error::from(CliCancelled::SecretInput)
    } else {
      error.into()
    }
  })?;
  if interactive {
    remove_one_line_ending(&mut value);
    eprintln!("Input received. Saving {key}...");
  }
  Ok(value)
}

pub fn remove_one_line_ending(value: &mut String) {
  if value.ends_with('\n') {
    value.pop();
    if value.ends_with('\r') {
      value.pop();
    }
  }
}
