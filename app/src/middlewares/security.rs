use crate::constants::api;
use axum::{
  http::{HeaderName, header},
  middleware::Next,
  response::Response,
};

/// Add browser hardening headers to every response and prevent API responses
/// from being stored by caches.
pub async fn headers(
  request: axum::extract::Request,
  next: Next,
) -> Response {
  let is_api = request.uri().path().starts_with(api::PREFIX);
  let mut response = next.run(request).await;
  let headers = response.headers_mut();
  let mut insert_if_absent = |name: HeaderName, value: &'static str| {
    headers
      .entry(name)
      .or_insert(axum::http::HeaderValue::from_static(value));
  };
  insert_if_absent(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
  insert_if_absent(header::X_FRAME_OPTIONS, "DENY");
  insert_if_absent(header::REFERRER_POLICY, "no-referrer");
  insert_if_absent(
    HeaderName::from_static("content-security-policy"),
    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self'; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'none'",
  );
  if is_api {
    insert_if_absent(header::CACHE_CONTROL, "no-store");
  }
  response
}
