use crate::modules::grants::model::GrantResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// An account, by id and email (an AI agent's name).
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountRef {
  pub id: String,
  pub email: String,
}

/// A named set of users that receives grants like a user does.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponse {
  /// Group identifier with `grp_` prefix.
  pub id: String,
  pub name: String,
  pub description: String,
  pub member_count: i64,
  /// Who created the group. Empty when that account is gone.
  pub created_by: Option<AccountRef>,
  pub created_at: String,
  pub updated_at: String,
}

/// A group in the list, with its members and grants.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupListItem {
  #[serde(flatten)]
  pub group: GroupResponse,
  /// Members by email.
  pub members: Vec<AccountRef>,
  pub grants: Vec<GrantResponse>,
}

/// A member of a group.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
  pub id: String,
  pub email: String,
  /// When the user joined the group.
  pub added_at: String,
  /// Who added them. Empty when that account is gone.
  pub added_by: Option<AccountRef>,
}

/// A group with its members and grants.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupDetailResponse {
  pub group: GroupResponse,
  pub members: Vec<GroupMember>,
  pub grants: Vec<GrantResponse>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupRequest {
  /// Unique name, at most 64 characters.
  pub name: String,
  #[serde(default)]
  pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGroupRequest {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
}
