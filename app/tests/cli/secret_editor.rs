#![cfg(unix)]

use app::cli::secret_editor::{Editor, MAX_EDITOR_BYTES, Session};
use std::{
  fs,
  os::unix::fs::{PermissionsExt, symlink},
};
use tempfile::TempDir;

#[tokio::test]
async fn sessions_accept_private_replacement_and_reject_unsafe_files() {
  let directory = TempDir::new().unwrap();
  let session = Session::new(&directory.path().join("sessions")).unwrap();
  session.write("A=original\n").unwrap();
  assert_eq!(
    fs::metadata(session.path()).unwrap().permissions().mode() & 0o777,
    0o700
  );
  assert_eq!(
    fs::metadata(session.file_path())
      .unwrap()
      .permissions()
      .mode()
      & 0o777,
    0o600
  );
  let replacement = session.path().join("replacement");
  fs::write(&replacement, "A=replaced\n").unwrap();
  fs::set_permissions(&replacement, fs::Permissions::from_mode(0o600)).unwrap();
  fs::rename(&replacement, session.file_path()).unwrap();
  assert_eq!(&*session.read().unwrap(), "A=replaced\n");
  fs::set_permissions(session.file_path(), fs::Permissions::from_mode(0o644)).unwrap();
  assert!(session.read().is_err());
  fs::set_permissions(session.file_path(), fs::Permissions::from_mode(0o600)).unwrap();
  fs::hard_link(session.file_path(), session.path().join("link")).unwrap();
  assert!(session.read().is_err());
  fs::remove_file(session.path().join("link")).unwrap();
  let file = fs::OpenOptions::new()
    .write(true)
    .open(session.file_path())
    .unwrap();
  file.set_len(MAX_EDITOR_BYTES + 1).unwrap();
  assert!(session.read().unwrap_err().to_string().contains("8 MiB"));
  drop(file);
  fs::remove_file(session.file_path()).unwrap();
  symlink(directory.path().join("outside"), session.file_path()).unwrap();
  assert!(session.read().is_err());
  fs::remove_file(session.file_path()).unwrap();
  assert!(session.read().is_err());
  fs::create_dir(session.file_path()).unwrap();
  assert!(session.read().is_err());
  let path = session.path().to_owned();
  session.close().await.unwrap();
  assert!(!path.exists());
}

#[tokio::test]
async fn scavenging_removes_abandoned_sessions_but_keeps_active_sessions() {
  let directory = TempDir::new().unwrap();
  let root = directory.path().join("sessions");
  let active = Session::new(&root).unwrap();
  active.write("A=private\n").unwrap();
  let stale = root.join("session-abandoned");
  fs::create_dir(&stale).unwrap();
  fs::set_permissions(&stale, fs::Permissions::from_mode(0o700)).unwrap();
  fs::write(stale.join("secrets.env"), "A=abandoned\n").unwrap();
  let outside = directory.path().join("outside");
  fs::create_dir(&outside).unwrap();
  fs::write(outside.join("keep"), "unrelated").unwrap();
  symlink(&outside, root.join("session-symlink")).unwrap();
  let next = Session::new(&root).unwrap();
  assert!(!stale.exists());
  assert_eq!(&*active.read().unwrap(), "A=private\n");
  assert!(outside.join("keep").exists());
  active.close().await.unwrap();
  next.close().await.unwrap();
}

#[tokio::test]
async fn protected_profiles_isolate_configuration_and_do_not_inherit_credentials() {
  let directory = TempDir::new().unwrap();
  let session = Session::new(&directory.path().join("sessions")).unwrap();
  for name in ["vim", "nvim", "hx", "nano"] {
    let executable = directory.path().join(name);
    fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let editor = Editor::from_command(executable.to_str().unwrap()).unwrap();
    assert!(editor.protected());
    let command = editor.command(&session).unwrap();
    let command = command.as_std();
    assert_eq!(command.get_current_dir(), Some(session.path()));
    let variables: Vec<_> = command
      .get_envs()
      .map(|(name, _)| name.to_string_lossy().into_owned())
      .collect();
    assert!(!variables.iter().any(|name| name.starts_with("DOPBASE_")
      || name == "UNRELATED_CREDENTIAL"
      || name == "HOME"));
    assert!(variables.contains(&"XDG_CONFIG_HOME".into()));
    let arguments: Vec<_> = command
      .get_args()
      .map(|arg| arg.to_string_lossy().into_owned())
      .collect();
    assert_eq!(
      arguments.last().unwrap(),
      &session.file_path().to_string_lossy()
    );
    match name {
      "vim" | "nvim" => assert!(
        arguments
          .contains(&"set noswapfile nobackup nowritebackup noundofile nomodeline noexrc".into())
      ),
      "nano" => assert!(arguments.contains(&"-R".into()) && arguments.contains(&"-I".into())),
      "hx" => assert!(
        fs::read_to_string(session.path().join("editor.toml"))
          .unwrap()
          .contains("enable = false")
      ),
      _ => unreachable!(),
    }
    let custom = Editor::from_command(&format!("'{}' --custom", executable.display())).unwrap();
    assert!(!custom.protected());
  }
  let pico = directory.path().join("pico");
  fs::write(&pico, "#!/bin/sh\nexit 0\n").unwrap();
  fs::set_permissions(&pico, fs::Permissions::from_mode(0o700)).unwrap();
  fs::remove_file(directory.path().join("nano")).unwrap();
  symlink(&pico, directory.path().join("nano")).unwrap();
  assert!(
    !Editor::from_command(directory.path().join("nano").to_str().unwrap())
      .unwrap()
      .protected()
  );
  assert!(Editor::from_command("'unterminated").is_err());
  assert!(Editor::from_command("").is_err());
  session.close().await.unwrap();
}
