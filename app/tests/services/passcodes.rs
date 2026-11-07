use app::services::{
  db::{DbClient, MIGRATOR},
  passcodes::{self, Consumed, PasscodePurpose},
};
use chrono::{Duration, Utc};

const LAUNCH: PasscodePurpose = PasscodePurpose::Launch;

async fn database() -> DbClient {
  let db = DbClient::connect("sqlite::memory:").await.unwrap();
  MIGRATOR.run(db.pool()).await.unwrap();
  for id in ["usr_root", "usr_other"] {
    sqlx::query("INSERT INTO admins(id,email,password_hash,role,is_root,created_at,updated_at) VALUES(?,?,'','admin',0,'t','t')")
      .bind(id)
      .bind(format!("{id}@dopbase.com"))
      .execute(db.pool())
      .await
      .unwrap();
  }
  db
}

async fn issue(
  db: &DbClient,
  admin_id: &str,
  code: &str,
  ttl: Duration,
) {
  let mut connection = db.pool().acquire().await.unwrap();
  passcodes::issue(
    &mut connection,
    LAUNCH,
    admin_id,
    code,
    ttl,
    Some("host"),
    Utc::now(),
  )
  .await
  .unwrap();
}

async fn consume(
  db: &DbClient,
  admin_id: &str,
  code: Option<&str>,
) -> Consumed {
  let mut tx = db.pool().begin().await.unwrap();
  let result = passcodes::consume(&mut tx, LAUNCH, admin_id, code, Utc::now())
    .await
    .unwrap();
  tx.commit().await.unwrap();
  result
}

async fn rows(db: &DbClient) -> i64 {
  sqlx::query_scalar("SELECT COUNT(*) FROM passcodes")
    .fetch_one(db.pool())
    .await
    .unwrap()
}

#[test]
fn launch_rules() {
  assert_eq!(LAUNCH.as_str(), "launch");
  assert_eq!(LAUNCH.default_ttl_seconds(), 60);
  assert_eq!(LAUNCH.ttl_range_seconds(), 15..=600);
  assert_eq!(LAUNCH.max_failures(), 5);
}

#[tokio::test]
async fn a_code_works_once() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Used
  );
  assert_eq!(rows(&db).await, 0);
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Missing
  );
}

#[tokio::test]
async fn only_the_hash_is_stored() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  let (purpose, hash): (String, Vec<u8>) =
    sqlx::query_as("SELECT purpose,code_hash FROM passcodes")
      .fetch_one(db.pool())
      .await
      .unwrap();
  assert_eq!(purpose, "launch");
  assert_eq!(hash.len(), 32);
  assert!(!hash.windows(8).any(|window| window == b"GJZGFLMT"));
}

#[tokio::test]
async fn issuing_again_replaces_the_code() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  issue(&db, "usr_root", "KTVBRMQD", Duration::seconds(60)).await;
  assert_eq!(rows(&db).await, 1);
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Wrong
  );
  assert_eq!(
    consume(&db, "usr_root", Some("KTVBRMQD")).await,
    Consumed::Used
  );
}

#[tokio::test]
async fn replacing_resets_wrong_tries() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  for _ in 0..4 {
    assert_eq!(consume(&db, "usr_root", None).await, Consumed::Wrong);
  }
  issue(&db, "usr_root", "KTVBRMQD", Duration::seconds(60)).await;
  assert_eq!(consume(&db, "usr_root", None).await, Consumed::Wrong);
  assert_eq!(
    consume(&db, "usr_root", Some("KTVBRMQD")).await,
    Consumed::Used
  );
}

#[tokio::test]
async fn codes_belong_to_their_account() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  issue(&db, "usr_other", "KTVBRMQD", Duration::seconds(60)).await;
  assert_eq!(
    consume(&db, "usr_other", Some("GJZGFLMT")).await,
    Consumed::Wrong
  );
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Used
  );
}

#[tokio::test]
async fn a_code_from_another_purpose_never_matches() {
  let db = database().await;
  // A row written for some other purpose with the same code, as a later
  // purpose such as OTP would store it.
  sqlx::query("INSERT INTO passcodes(id,purpose,admin_id,code_hash,created_at,expires_at) VALUES('psc_x','otp','usr_root',?,?,?)")
    .bind(app::services::token::hash("otp:GJZGFLMT"))
    .bind(Utc::now().to_rfc3339())
    .bind((Utc::now() + Duration::seconds(60)).to_rfc3339())
    .execute(db.pool())
    .await
    .unwrap();
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Missing
  );
  issue(&db, "usr_root", "KTVBRMQD", Duration::seconds(60)).await;
  assert_eq!(rows(&db).await, 2, "one row per purpose");
}

#[tokio::test]
async fn the_fifth_wrong_try_burns_the_code() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  for _ in 0..4 {
    assert_eq!(
      consume(&db, "usr_root", Some("KTVBRMQD")).await,
      Consumed::Wrong
    );
  }
  assert_eq!(
    consume(&db, "usr_root", Some("KTVBRMQD")).await,
    Consumed::Burned
  );
  assert_eq!(rows(&db).await, 0);
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Missing
  );
}

#[tokio::test]
async fn an_expired_code_is_deleted_when_presented() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(-1)).await;
  assert_eq!(
    consume(&db, "usr_root", Some("GJZGFLMT")).await,
    Consumed::Expired
  );
  assert_eq!(rows(&db).await, 0);
}

#[tokio::test]
async fn sweeping_deletes_only_expired_codes() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(-1)).await;
  issue(&db, "usr_other", "KTVBRMQD", Duration::seconds(60)).await;
  let mut connection = db.pool().acquire().await.unwrap();
  assert_eq!(
    passcodes::sweep_expired(&mut connection, Utc::now())
      .await
      .unwrap(),
    1
  );
  drop(connection);
  assert_eq!(
    consume(&db, "usr_other", Some("KTVBRMQD")).await,
    Consumed::Used
  );
}

#[tokio::test]
async fn deleting_the_account_deletes_its_codes() {
  let db = database().await;
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  sqlx::query("DELETE FROM admins WHERE id='usr_root'")
    .execute(db.pool())
    .await
    .unwrap();
  assert_eq!(rows(&db).await, 0);
}

#[tokio::test]
async fn racing_requests_use_a_code_once() {
  let directory = tempfile::TempDir::new().unwrap();
  let url = format!("sqlite://{}", directory.path().join("race.db").display());
  let db = DbClient::connect(&url).await.unwrap();
  MIGRATOR.run(db.pool()).await.unwrap();
  sqlx::query("INSERT INTO admins(id,email,password_hash,role,is_root,created_at,updated_at) VALUES('usr_root','root@dopbase.com','','admin',1,'t','t')")
    .execute(db.pool())
    .await
    .unwrap();
  issue(&db, "usr_root", "GJZGFLMT", Duration::seconds(60)).await;
  let attempts = (0..8).map(|_| {
    let pool = db.pool().clone();
    tokio::spawn(async move {
      let mut tx = pool.begin().await.unwrap();
      let result = passcodes::consume(&mut tx, LAUNCH, "usr_root", Some("GJZGFLMT"), Utc::now())
        .await
        .unwrap();
      tx.commit().await.unwrap();
      result
    })
  });
  let mut used = 0;
  for attempt in attempts.collect::<Vec<_>>() {
    if attempt.await.unwrap() == Consumed::Used {
      used += 1;
    }
  }
  assert_eq!(used, 1);
  db.close().await;
}
