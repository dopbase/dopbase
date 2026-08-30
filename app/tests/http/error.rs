use std::collections::BTreeMap;

use app::constants::errors::{EMAIL_INVAILD, EMAIL_INVAILD_MESSAGE};
use app::http::error::ErrorBody;

#[test]
fn serializes_required_shape() {
  let body = ErrorBody {
    success: false,
    error: BTreeMap::from([(EMAIL_INVAILD.into(), EMAIL_INVAILD_MESSAGE.into())]),
  };
  assert_eq!(
    serde_json::to_value(body).unwrap(),
    serde_json::json!({"success":false,"error":{"EMAIL_INVAILD":"Please use proper email"}})
  );
}
