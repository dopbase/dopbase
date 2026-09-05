use std::fs;

use app::services::crypto::{CryptoService, EncryptedValue, parse_master_key, rekey_database};
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

#[test]
fn parses_raw_and_hex_master_keys() {
  let raw = vec![0x42_u8; 32];
  assert_eq!(parse_master_key(&raw).unwrap(), raw);

  let hex = hex::encode(&raw);
  assert_eq!(parse_master_key(hex.as_bytes()).unwrap(), raw);
  assert_eq!(
    parse_master_key(format!("\n  {hex}  \n").as_bytes()).unwrap(),
    raw
  );

  assert!(parse_master_key(b"too_short").is_err());
  assert!(
    parse_master_key(b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefg").is_err()
  );
}

#[test]
fn cross_key_backup_decryption_requires_the_source_key() {
  let source_key = vec![0x11_u8; 32];
  let target_crypto = CryptoService::from_key(vec![0x22_u8; 32]);
  let source_crypto = CryptoService::from_key(source_key.clone());
  let payload = b"secret-dopbase-backup-payload-content";
  let encrypted = source_crypto.encrypt_backup(payload).unwrap();

  assert!(target_crypto.decrypt_backup(&encrypted).is_err());
  assert_eq!(
    CryptoService::decrypt_backup_with_key(&encrypted, &source_key).unwrap(),
    payload
  );
}

#[tokio::test]
async fn rekey_database_preserves_values_and_swaps_keys() {
  const VERIFICATION_TEXT: &[u8] = b"dopbase-master-key-v1";
  let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();

  sqlx::query(
    "CREATE TABLE instance_metadata (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      verification_ciphertext BLOB NOT NULL,
      verification_nonce BLOB NOT NULL,
      created_at TEXT NOT NULL
    );",
  )
  .execute(&pool)
  .await
  .unwrap();
  sqlx::query(
    "CREATE TABLE secrets (
      environment_id TEXT NOT NULL,
      key TEXT NOT NULL,
      version INTEGER NOT NULL DEFAULT 1,
      ciphertext BLOB NOT NULL,
      value_nonce BLOB NOT NULL,
      wrapped_key BLOB NOT NULL,
      key_nonce BLOB NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      PRIMARY KEY(environment_id, key)
    );",
  )
  .execute(&pool)
  .await
  .unwrap();

  let old_key = vec![0x11_u8; 32];
  let new_key = vec![0x22_u8; 32];
  let old_crypto = CryptoService::from_key(old_key.clone());
  let new_crypto = CryptoService::from_key(new_key.clone());
  let (verification_ciphertext, verification_nonce) = old_crypto
    .encrypt_master(VERIFICATION_TEXT, b"instance-verification")
    .unwrap();
  sqlx::query(
    "INSERT INTO instance_metadata (id, verification_ciphertext, verification_nonce, created_at) VALUES (1, ?, ?, ?)",
  )
  .bind(verification_ciphertext)
  .bind(verification_nonce)
  .bind("2026-09-05T00:00:00Z")
  .execute(&pool)
  .await
  .unwrap();

  let cleartext = b"my-super-secret-password";
  let encrypted = old_crypto
    .encrypt(cleartext, "env_test", "DB_PASS", 1)
    .unwrap();
  sqlx::query(
    "INSERT INTO secrets (environment_id, key, version, ciphertext, value_nonce, wrapped_key, key_nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
  )
  .bind("env_test")
  .bind("DB_PASS")
  .bind(1_i64)
  .bind(&encrypted.ciphertext)
  .bind(&encrypted.value_nonce)
  .bind(&encrypted.wrapped_key)
  .bind(&encrypted.key_nonce)
  .bind("2026-09-05T00:00:00Z")
  .bind("2026-09-05T00:00:00Z")
  .execute(&pool)
  .await
  .unwrap();

  assert!(
    new_crypto
      .decrypt(&encrypted, "env_test", "DB_PASS", 1)
      .is_err()
  );
  rekey_database(&pool, &old_key, &new_key).await.unwrap();

  let metadata: (Vec<u8>, Vec<u8>) = sqlx::query_as(
    "SELECT verification_ciphertext, verification_nonce FROM instance_metadata WHERE id = 1",
  )
  .fetch_one(&pool)
  .await
  .unwrap();
  assert_eq!(
    new_crypto
      .decrypt_master(&metadata.0, &metadata.1, b"instance-verification")
      .unwrap(),
    VERIFICATION_TEXT
  );

  let row: (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) = sqlx::query_as(
    "SELECT ciphertext, value_nonce, wrapped_key, key_nonce FROM secrets WHERE environment_id = ? AND key = ?",
  )
  .bind("env_test")
  .bind("DB_PASS")
  .fetch_one(&pool)
  .await
  .unwrap();
  let rekeyed = EncryptedValue {
    ciphertext: row.0,
    value_nonce: row.1,
    wrapped_key: row.2,
    key_nonce: row.3,
  };
  assert_eq!(
    new_crypto
      .decrypt(&rekeyed, "env_test", "DB_PASS", 1)
      .unwrap()
      .as_slice(),
    cleartext
  );
}
