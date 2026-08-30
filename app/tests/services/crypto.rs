use std::fs;

use app::services::crypto::CryptoService;
use app::services::db::DbClient;

#[tokio::test]
async fn round_trip_detects_tampering_and_wrong_master_key() {
  let directory = tempfile::TempDir::new().unwrap();
  let database = directory.path().join("crypto.db");
  let key = directory.path().join("master.key");
  let db = DbClient::connect(&format!("sqlite://{}", database.display()))
    .await
    .unwrap();
  db.migrate().await.unwrap();
  let crypto = CryptoService::initialize(db.pool(), &key).await.unwrap();
  let mut encrypted = crypto
    .encrypt(b"private-value", "env_01", "DATABASE_URL", 1)
    .unwrap();
  assert_eq!(
    crypto
      .decrypt(&encrypted, "env_01", "DATABASE_URL", 1)
      .unwrap()
      .as_slice(),
    b"private-value"
  );
  encrypted.ciphertext[0] ^= 1;
  assert!(
    crypto
      .decrypt(&encrypted, "env_01", "DATABASE_URL", 1)
      .is_err()
  );

  fs::write(&key, [7_u8; 32]).unwrap();
  assert!(CryptoService::initialize(db.pool(), &key).await.is_err());
  db.close().await;
}

#[cfg(unix)]
#[tokio::test]
async fn generated_key_is_owner_only() {
  use std::os::unix::fs::PermissionsExt;
  let directory = tempfile::TempDir::new().unwrap();
  let database = directory.path().join("permissions.db");
  let key = directory.path().join("master.key");
  let db = DbClient::connect(&format!("sqlite://{}", database.display()))
    .await
    .unwrap();
  db.migrate().await.unwrap();
  CryptoService::initialize(db.pool(), &key).await.unwrap();
  assert_eq!(
    fs::metadata(key).unwrap().permissions().mode() & 0o777,
    0o600
  );
  db.close().await;
}
