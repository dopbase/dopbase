use crate::constants::tokens::ENVIRONMENT_ID_PREFIX;

pub const FIRST_ENVIRONMENT_NUMBER: u32 = 1_000;
pub const LAST_ENVIRONMENT_NUMBER: u32 = 999_999;

pub fn from_number(number: u32) -> String {
  format!("{ENVIRONMENT_ID_PREFIX}{number}")
}
