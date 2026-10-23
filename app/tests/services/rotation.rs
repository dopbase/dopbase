use app::services::rotation::{default_grace, grace_duration, max_grace_seconds};

#[test]
fn grace_accepts_zero_bounded_hours_and_days() {
  assert_eq!(grace_duration("0").unwrap().num_seconds(), 0);
  assert_eq!(grace_duration("1h").unwrap().num_hours(), 1);
  assert_eq!(grace_duration("24h").unwrap().num_hours(), 24);
  assert_eq!(grace_duration("720h").unwrap().num_hours(), 720);
  assert_eq!(grace_duration("7d").unwrap().num_days(), 7);
  assert_eq!(grace_duration("30d").unwrap().num_days(), 30);
  assert_eq!(
    grace_duration("30d").unwrap().num_seconds(),
    max_grace_seconds()
  );
  assert_eq!(default_grace().num_hours(), 24);
}

#[test]
fn grace_rejects_units_overflow_and_negative_values() {
  for value in [
    "",
    "0h",
    "30m",
    "-1h",
    "1.5h",
    "1y",
    "721h",
    "31d",
    "never",
    " 24h",
    "24h ",
    "999999999999999999999d",
  ] {
    assert!(grace_duration(value).is_err(), "accepted {value:?}");
  }
}
