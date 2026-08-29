use super::{model::InstanceStatus, repository};
use crate::{extractors::require_admin, http::HttpError, models::AuthIdentity, state::AppState};
pub async fn status(
    state: &AppState,
    identity: &AuthIdentity,
) -> Result<InstanceStatus, HttpError> {
    require_admin(identity)?;
    state.db.ping().await?;
    Ok(InstanceStatus {
        version: env!("CARGO_PKG_VERSION"),
        public_url: state.config.public_url.clone(),
        initialization_state: if repository::initialized(state.db.pool()).await? {
            "ready"
        } else {
            "setupRequired"
        },
        database_health: "healthy",
        key_availability: "available",
        configuration_reload: "restartRequired",
    })
}
