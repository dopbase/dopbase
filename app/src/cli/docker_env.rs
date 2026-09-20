use crate::models::SecretInput;
use anyhow::{Result, bail};

// Docker env-file entries are single-line KEY=VALUE pairs. Keep each rendered
// line below 65535 bytes, which matches Docker's single-line env-file parser
// safety limit. Larger values are rejected before export so the file remains
// parseable by `docker run --env-file` / `docker exec --env-file`.
// This is separate from the OS-level total environment size limit enforced by
// execve() on Linux.
const MAX_DOCKER_ENV_LINE_BYTES: usize = 65_535;

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
    if entry.key.len() + 1 + entry.value.len() > MAX_DOCKER_ENV_LINE_BYTES {
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
