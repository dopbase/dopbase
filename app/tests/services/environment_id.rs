use app::services::environment_id::from_number;

#[test]
fn environment_ids_expand_only_at_decimal_boundaries() {
  assert_eq!(from_number(1_000), "env_1000");
  assert_eq!(from_number(9_999), "env_9999");
  assert_eq!(from_number(10_000), "env_10000");
  assert_eq!(from_number(99_999), "env_99999");
  assert_eq!(from_number(100_000), "env_100000");
  assert_eq!(from_number(999_999), "env_999999");
}
