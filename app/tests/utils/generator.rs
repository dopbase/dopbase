use app::utils::generator::{environment_id, from_number};

#[test]
fn environment_ids_are_zero_padded_to_six_digits() {
  assert_eq!(from_number(1), "env_000001");
  assert_eq!(from_number(9_999), "env_009999");
  assert_eq!(from_number(10_000), "env_010000");
  assert_eq!(from_number(99_999), "env_099999");
  assert_eq!(from_number(100_000), "env_100000");
  assert_eq!(from_number(999_999), "env_999999");
}

#[test]
fn generated_environment_ids_stay_in_the_supported_range() {
  for _ in 0..100 {
    let id = environment_id().unwrap();
    assert_eq!(id.len(), 10);
    assert!(id.starts_with("env_"));
    let number = id[4..].parse::<u32>().unwrap();
    assert!((1..=999_999).contains(&number));
  }
}
