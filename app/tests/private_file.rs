use std::sync::{Arc, Barrier};

#[test]
fn replace_writer_overwrites_existing_destination() {
  let directory = tempfile::TempDir::new().unwrap();
  let path = directory.path().join("config.toml");
  std::fs::write(&path, b"old").unwrap();

  app::utils::private_file::write(&path, b"new", true).unwrap();

  assert_eq!(std::fs::read(path).unwrap(), b"new");
}

#[test]
fn concurrent_no_replace_writers_never_overwrite_each_other() {
  let directory = tempfile::TempDir::new().unwrap();
  let path = directory.path().join("secrets.env");
  let barrier = Arc::new(Barrier::new(2));
  let mut writers = Vec::new();
  for contents in [b"first".as_slice(), b"second".as_slice()] {
    let path = path.clone();
    let barrier = barrier.clone();
    let contents = contents.to_vec();
    writers.push(std::thread::spawn(move || {
      barrier.wait();
      app::utils::private_file::write(&path, &contents, false)
    }));
  }
  let outcomes = writers
    .into_iter()
    .map(|writer| writer.join().unwrap().is_ok())
    .collect::<Vec<_>>();
  assert_eq!(outcomes.iter().filter(|success| **success).count(), 1);
  let stored = std::fs::read(path).unwrap();
  assert!(stored == b"first" || stored == b"second");
}

#[test]
fn concurrent_create_if_missing_writers_preserve_one_complete_file() {
  let directory = tempfile::TempDir::new().unwrap();
  let path = directory.path().join("config.toml");
  let barrier = Arc::new(Barrier::new(8));
  let writers = (0..8)
    .map(|index| {
      let path = path.clone();
      let barrier = barrier.clone();
      std::thread::spawn(move || {
        barrier.wait();
        app::utils::private_file::write_if_missing(&path, format!("writer-{index}").as_bytes())
          .unwrap()
      })
    })
    .collect::<Vec<_>>();
  let created = writers
    .into_iter()
    .map(|writer| writer.join().unwrap())
    .filter(|created| *created)
    .count();
  assert_eq!(created, 1);
  let saved = std::fs::read(&path).unwrap();
  assert!(
    String::from_utf8(saved.clone())
      .unwrap()
      .starts_with("writer-")
  );
  assert!(!app::utils::private_file::write_if_missing(&path, b"replacement").unwrap());
  assert_eq!(std::fs::read(&path).unwrap(), saved);
  assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn create_if_missing_reports_other_write_errors_with_the_path() {
  let directory = tempfile::TempDir::new().unwrap();
  let parent = directory.path().join("not-a-directory");
  std::fs::write(&parent, b"keep").unwrap();
  let path = parent.join("config.toml");
  let error = app::utils::private_file::write_if_missing(&path, b"reference").unwrap_err();
  assert!(format!("{error:#}").contains(&path.display().to_string()));
  assert_eq!(std::fs::read(parent).unwrap(), b"keep");
}
