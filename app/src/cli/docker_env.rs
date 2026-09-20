use crate::models::SecretInput;
use anyhow::{Result, bail};

const MAX_LINE_CONTENT_BYTES: usize = 65_535;

pub fn render(entries: &[SecretInput]) -> Result<String> {
  let mut output = String::new();
  for entry in entries {
    if entry.value.contains(['\n', '\r']) {
      bail!(
        "secret {:?} contains a line break, which Docker env files cannot represent",
        entry.key
      );
    }
    if entry.value.contains('\0') {
      bail!(
        "secret {:?} contains a NUL byte, which Docker environment variables cannot represent",
        entry.key
      );
    }
    if entry.key.len() + 1 + entry.value.len() > MAX_LINE_CONTENT_BYTES {
      bail!(
        "secret {:?} exceeds Docker's maximum env-file line length",
        entry.key
      );
    }
    output.push_str(&entry.key);
    output.push('=');
    output.push_str(&entry.value);
    output.push('\n');
  }
  Ok(output)
}
