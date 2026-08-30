use super::model::HealthResponse;
pub fn health() -> HealthResponse {
  HealthResponse {
    product: "dopbase",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "v1",
    status: "ok",
  }
}
