/// Normalize CLI path-like arguments (Git Bash converts `/api` to `…/Git/api` on Windows).
pub fn normalize(reference: &str) -> String {
  let reference = reference.trim();
  let reference = if reference.starts_with("//") && !reference.starts_with("///") {
    format!("/{}", reference.trim_start_matches('/'))
  } else {
    reference.to_owned()
  };
  unwrap_git_bash_root_path(&reference)
}

fn unwrap_git_bash_root_path(reference: &str) -> String {
  let normalized = reference.replace('\\', "/");
  const MARKER: &str = "/Git/";
  if let Some(idx) = normalized.find(MARKER) {
    let suffix = normalized[idx + MARKER.len()..].trim_start_matches('/');
    if !suffix.is_empty() {
      return format!("/{}", suffix);
    }
  }
  reference.to_owned()
}
