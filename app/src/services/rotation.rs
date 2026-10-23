use chrono::Duration;

/// Grace period used when a rotation does not request one. The previous value
/// stays valid for one day so consumers can pick up the new value.
pub const DEFAULT_GRACE_HOURS: i64 = 24;
/// Longest supported rotation grace period.
pub const MAX_GRACE_DAYS: i64 = 30;
pub const GRACE_ERROR: &str = "Use 0 or a whole number of hours or days, up to 30 days.";

/// Parses a rotation grace period: `0` retires the previous value
/// immediately, otherwise a positive whole number of hours (`24h`) or
/// days (`7d`).
pub fn grace_duration(value: &str) -> Result<Duration, &'static str> {
  if value == "0" {
    return Ok(Duration::zero());
  }
  let (digits, unit) = if let Some(digits) = value.strip_suffix('h') {
    (digits, "h")
  } else if let Some(digits) = value.strip_suffix('d') {
    (digits, "d")
  } else {
    return Err(GRACE_ERROR);
  };
  if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
    return Err(GRACE_ERROR);
  }
  let amount: i64 = digits.parse().map_err(|_| GRACE_ERROR)?;
  let max = match unit {
    "h" => MAX_GRACE_DAYS * 24,
    "d" => MAX_GRACE_DAYS,
    _ => return Err(GRACE_ERROR),
  };
  if amount == 0 || amount > max {
    return Err(GRACE_ERROR);
  }
  Ok(if unit == "h" {
    Duration::hours(amount)
  } else {
    Duration::days(amount)
  })
}

/// Grace period applied when a rotation request carries no explicit value.
pub fn default_grace() -> Duration {
  Duration::hours(DEFAULT_GRACE_HOURS)
}

/// Longest grace period a rotation request may ask for, in seconds.
pub fn max_grace_seconds() -> i64 {
  MAX_GRACE_DAYS * 24 * 60 * 60
}
