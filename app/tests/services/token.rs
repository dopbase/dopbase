use app::services::token::expiry_duration;

#[test]
fn token_expiry_accepts_only_bounded_hours_days_or_never() {
  assert_eq!(expiry_duration("never").unwrap(), None);
  assert_eq!(expiry_duration("1h").unwrap().unwrap().num_hours(), 1);
  assert_eq!(
    expiry_duration("26280h").unwrap().unwrap().num_hours(),
    26280
  );
  assert_eq!(expiry_duration("1095d").unwrap().unwrap().num_days(), 1095);
  for value in [
    "",
    "0h",
    "-1d",
    "1.5h",
    "30m",
    "1y",
    "26281h",
    "1096d",
    "999999999999999999999d",
    " 1h",
  ] {
    assert!(expiry_duration(value).is_err(), "accepted {value:?}");
  }
}
