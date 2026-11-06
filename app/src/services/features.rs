//! Features the server can turn on or off as a whole.
//!
//! A feature is wider than a capability: a capability says what one role may
//! do, a feature says whether the instance offers that kind of thing at all.
//! A disabled feature takes its capabilities away from every account, root
//! included, and its routes answer `FEATURE_DISABLED`. Turning a feature off
//! never deletes data, so turning it back on restores everything.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
  constants::{
    config::{ENV_RBAC, ENV_RBAC_ROLES, ENV_SSO},
    errors::FEATURE_DISABLED,
  },
  http::HttpError,
  services::authx::Capability,
};

/// A part of Dopbase the server can turn off.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq, PartialOrd, Ord)]
pub enum Feature {
  /// Single sign-on with an OpenID Connect provider.
  #[serde(rename = "sso")]
  Sso,
  /// Adding and changing people, groups and their grants.
  #[serde(rename = "rbac")]
  Rbac,
  /// Custom roles. Needs `rbac`.
  #[serde(rename = "rbac.roles")]
  RbacRoles,
}

impl Feature {
  pub const ALL: [Self; 3] = [Self::Sso, Self::Rbac, Self::RbacRoles];

  /// The environment variable that turns the feature on.
  pub fn setting(self) -> &'static str {
    match self {
      Self::Sso => ENV_SSO,
      Self::Rbac => ENV_RBAC,
      Self::RbacRoles => ENV_RBAC_ROLES,
    }
  }

  /// Capabilities that mean nothing while the feature is off.
  fn capabilities(self) -> &'static [Capability] {
    match self {
      Self::Sso => &[Capability::SsoManage],
      // roles.manage sits under RBAC roles alone: they are off whenever
      // RBAC is.
      Self::Rbac => &[Capability::GroupsManage, Capability::GrantsManage],
      Self::RbacRoles => &[Capability::RolesManage],
    }
  }
}

/// Which features this server offers. Every feature is off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Features {
  pub sso: bool,
  pub rbac: bool,
  /// Counts only while `rbac` is on.
  pub rbac_roles: bool,
}

impl Features {
  /// Every feature on.
  pub const fn all() -> Self {
    Self {
      sso: true,
      rbac: true,
      rbac_roles: true,
    }
  }

  pub fn enabled(
    &self,
    feature: Feature,
  ) -> bool {
    match feature {
      Feature::Sso => self.sso,
      Feature::Rbac => self.rbac,
      Feature::RbacRoles => self.rbac && self.rbac_roles,
    }
  }

  /// The features that are on, for the session response.
  pub fn list(&self) -> Vec<Feature> {
    Feature::ALL
      .into_iter()
      .filter(|feature| self.enabled(*feature))
      .collect()
  }

  /// True when custom roles are asked for without RBAC, so they stay off.
  pub fn rbac_roles_ignored(&self) -> bool {
    self.rbac_roles && !self.rbac
  }

  /// Whether `capability` belongs to a feature that's off, so no account
  /// holds it.
  pub fn blocks(
    &self,
    capability: Capability,
  ) -> bool {
    Feature::ALL
      .into_iter()
      .any(|feature| !self.enabled(feature) && feature.capabilities().contains(&capability))
  }

  /// Refuse the request when `feature` is off.
  pub fn require(
    &self,
    feature: Feature,
  ) -> Result<(), HttpError> {
    if self.enabled(feature) {
      return Ok(());
    }
    Err(disabled(feature))
  }
}

/// The error a disabled feature answers with.
pub fn disabled(feature: Feature) -> HttpError {
  let setting = feature.setting();
  let message = match feature {
    Feature::Sso => {
      format!("Single sign-on is off on this server. Set {setting}=true to turn it on.")
    }
    Feature::Rbac => {
      format!("Managing people and groups is off on this server. Set {setting}=true to turn it on.")
    }
    Feature::RbacRoles => format!(
      "Custom roles are off on this server. Set {}=true and {setting}=true to turn them on.",
      Feature::Rbac.setting()
    ),
  };
  HttpError::forbidden(FEATURE_DISABLED, &message)
}
