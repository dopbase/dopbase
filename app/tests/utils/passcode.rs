use app::utils::passcode::{ALPHABET, LENGTH, display, generate, normalize};

#[test]
fn generated_codes_are_eight_consonants() {
  for _ in 0..200 {
    let code = generate().unwrap();
    assert_eq!(code.len(), LENGTH);
    assert!(code.bytes().all(|byte| ALPHABET.contains(&byte)), "{code}");
  }
}

#[test]
fn generated_codes_differ() {
  let codes: std::collections::HashSet<_> = (0..50).map(|_| generate().unwrap()).collect();
  assert_eq!(codes.len(), 50);
}

#[test]
fn normalize_accepts_dashes_spaces_and_lowercase() {
  for typed in [
    "GJZG-FLMT",
    "gjzg-flmt",
    "gjzg flmt",
    "GJZGFLMT",
    " gjzg - flmt ",
  ] {
    assert_eq!(normalize(typed).as_deref(), Some("GJZGFLMT"), "{typed}");
  }
}

#[test]
fn normalize_refuses_values_that_cannot_be_codes() {
  for typed in [
    "",
    "GJZG-FLM",
    "GJZG-FLMTX",
    "GJZA-FLMT",
    "GJZG-FLM1",
    "GJZG_FLMT",
  ] {
    assert_eq!(normalize(typed), None, "{typed}");
  }
}

#[test]
fn display_splits_the_code_in_two() {
  assert_eq!(display("GJZGFLMT"), "GJZG-FLMT");
  let code = generate().unwrap();
  assert_eq!(normalize(&display(&code)), Some(code));
}
