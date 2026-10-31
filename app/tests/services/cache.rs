use app::services::cache::RateLimiter;

async fn fail(
  limiter: &RateLimiter,
  key: &str,
  times: u32,
) {
  for _ in 0..times {
    limiter.failure(key).await;
  }
}

#[tokio::test]
async fn five_failures_lock_a_key_until_it_is_cleared() {
  let limiter = RateLimiter::default();
  fail(&limiter, "login:victim@lab.invalid", 4).await;
  assert!(limiter.check("login:victim@lab.invalid").await);
  fail(&limiter, "login:victim@lab.invalid", 1).await;
  assert!(!limiter.check("login:victim@lab.invalid").await);
  assert!(limiter.check("login:other@lab.invalid").await);

  limiter.clear("login:victim@lab.invalid").await;
  assert!(limiter.check("login:victim@lab.invalid").await);
}

#[tokio::test]
async fn remaining_tries_and_wait_follow_the_failures() {
  let limiter = RateLimiter::default();
  let key = "login:victim@lab.invalid";
  assert_eq!(limiter.remaining(key).await, 5);
  assert_eq!(limiter.retry_after(key).await, None);
  for left in (0..5).rev() {
    fail(&limiter, key, 1).await;
    assert_eq!(limiter.remaining(key).await, left);
  }
  let wait = limiter.retry_after(key).await.unwrap();
  assert!(wait <= RateLimiter::window());
  assert!(wait > RateLimiter::window() - std::time::Duration::from_secs(60));
  assert_eq!(limiter.remaining("login:other@lab.invalid").await, 5);

  limiter.clear(key).await;
  assert_eq!(limiter.remaining(key).await, 5);
  assert_eq!(limiter.retry_after(key).await, None);
}

#[tokio::test]
async fn checks_alone_do_not_take_buckets() {
  let limiter = RateLimiter::with_capacity(10, 100);
  fail(&limiter, "victim", 5).await;
  for index in 0..1_000 {
    assert!(limiter.check(&format!("probe-{index}")).await);
  }
  assert!(!limiter.check("victim").await);
}

// GHSA-q8mx-j92m-jq92: failed logins for made-up emails used to evict the
// oldest bucket, which is the locked one, and reset the lockout.
#[tokio::test]
async fn flooding_new_keys_does_not_reset_a_lockout() {
  let limiter = RateLimiter::default();
  fail(&limiter, "login:victim@lab.invalid", 5).await;
  for index in 0..10_600 {
    let key = format!("login:f{index}@lab.invalid");
    assert!(limiter.check(&key).await);
    limiter.failure(&key).await;
  }
  assert!(!limiter.check("login:victim@lab.invalid").await);
}

#[tokio::test]
async fn flooding_new_keys_does_not_reset_a_partly_used_budget() {
  let limiter = RateLimiter::with_capacity(100, 1_000);
  fail(&limiter, "victim", 4).await;
  for index in 0..1_000 {
    limiter.failure(&format!("flood-{index}")).await;
  }
  assert!(limiter.check("victim").await);
  fail(&limiter, "victim", 1).await;
  assert!(!limiter.check("victim").await);
}

#[tokio::test]
async fn new_keys_are_refused_once_locked_buckets_fill_the_hard_cap() {
  let limiter = RateLimiter::with_capacity(4, 8);
  for index in 0..8 {
    fail(&limiter, &format!("locked-{index}"), 5).await;
  }
  assert!(!limiter.check("newcomer").await);
  for index in 0..8 {
    assert!(!limiter.check(&format!("locked-{index}")).await);
  }

  limiter.clear("locked-0").await;
  assert!(limiter.check("newcomer").await);
}
