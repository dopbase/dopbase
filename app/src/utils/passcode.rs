//! Short codes a person reads or types: the user code of a browser-approved
//! CLI login, and the passcodes in the `passcodes` table. Eight letters,
//! shown as `XXXX-XXXX`.

/// Consonants only (RFC 8628 section 6.1): no vowels to spell words, and
/// nothing that reads as another character.
pub const ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
pub const LENGTH: usize = 8;

/// A new code in its canonical form: eight uppercase letters, no dash.
pub fn generate() -> Result<String, getrandom::Error> {
  let mut code = String::with_capacity(LENGTH);
  // Rejection sampling keeps every letter equally likely: 240 is the
  // largest multiple of the alphabet size that fits in a byte.
  let limit = (256 / ALPHABET.len() * ALPHABET.len()) as u8;
  while code.len() < LENGTH {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)?;
    for byte in bytes {
      if byte < limit && code.len() < LENGTH {
        code.push(ALPHABET[byte as usize % ALPHABET.len()] as char);
      }
    }
  }
  Ok(code)
}

/// `WDJB-MJHT`, `wdjb mjht`, and `WDJBMJHT` are the same code. Returns the
/// canonical form, or `None` when the value cannot be a code.
pub fn normalize(value: &str) -> Option<String> {
  let code: String = value
    .chars()
    .filter(|character| !matches!(character, '-' | ' '))
    .map(|character| character.to_ascii_uppercase())
    .collect();
  (code.len() == LENGTH && code.bytes().all(|byte| ALPHABET.contains(&byte))).then_some(code)
}

/// The canonical form split in two for reading: `WDJB-MJHT`.
pub fn display(code: &str) -> String {
  format!("{}-{}", &code[..4], &code[4..])
}
