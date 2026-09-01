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
