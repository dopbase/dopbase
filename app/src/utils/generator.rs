use crate::constants::tokens::ENVIRONMENT_ID_PREFIX;

pub const LAST_ENVIRONMENT_NUMBER: u32 = 999_999;
const ENVIRONMENT_NUMBER_COUNT: u64 = LAST_ENVIRONMENT_NUMBER as u64;

pub fn from_number(number: u32) -> String {
  format!("{ENVIRONMENT_ID_PREFIX}{number:06}")
}

pub fn environment_id() -> anyhow::Result<String> {
  let sample_space = u32::MAX as u64 + 1;
  let unbiased_limit = sample_space - sample_space % ENVIRONMENT_NUMBER_COUNT;
  loop {
    let mut bytes = [0_u8; 4];
    getrandom::fill(&mut bytes)?;
    let value = u32::from_le_bytes(bytes) as u64;
    if value < unbiased_limit {
      return Ok(from_number((value % ENVIRONMENT_NUMBER_COUNT + 1) as u32));
    }
  }
}
