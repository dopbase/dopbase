use super::{
  model::*,
  repository::{self, Account, NewUser, UserRow},
};
use crate::{
  constants::errors::AUTHORIZATION_DENIED,
  constants::errors::PASSWORD_REQUIRED,
  constants::errors::USER_LIMIT_REACHED,
  extractors::{
    require_browser, require_capability_browser, require_mutation, require_recent_browser_auth,
  },
  http::HttpError,
  models::{AdminRole, AuthIdentity},
  modules::{common, grants, grants::model::GroupRef, tokens},
  services::{
    authx::{self, Access, Capability, Catalog},
    features::Feature,
    token,
  },
  state::AppState,
};
use axum::http::HeaderMap;
use chrono::Utc;
use std::collections::HashMap;

fn actor(identity: &AuthIdentity) -> (&str, &str) {
  match identity {
    AuthIdentity::Admin {
      admin_id, email, ..
    } => (admin_id, email),
    _ => ("", ""),
  }
}

/// The role an account gets, picked by id or by its fixed name.
struct ChosenRole {
  id: String,
  /// The legacy `admins.role` value.
  legacy: AdminRole,
  /// Holders sign in with personal tokens only, like AI agents.
  token_only: bool,
  builtin: bool,
}

async fn choose_role(
  state: &AppState,
  role: Option<AdminRole>,
  role_id: Option<&str>,
) -> Result<ChosenRole, HttpError> {
  let id = match (role_id, role.unwrap_or(AdminRole::Admin)) {
    (Some(id), _) => id,
    (None, AdminRole::Root) => {
      return Err(HttpError::forbidden(
        "ROOT_PROTECTED",
        "The root role is reserved for the setup account.",
      ));
    }
    (None, AdminRole::Admin) => "rol_admin",
    (None, AdminRole::Member) => "rol_member",
    (None, AdminRole::AiAgent) => "rol_ai_agent",
  };
  let (token_only, builtin) = repository::role_sign_in(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::bad_request("ROLE_INVALID", "The role was not found."))?;
  let legacy = match id {
    "rol_admin" => AdminRole::Admin,
    _ if token_only => AdminRole::AiAgent,
    _ => AdminRole::Member,
  };
  Ok(ChosenRole {
    id: id.to_owned(),
    legacy,
    token_only,
    builtin,
  })
}

/// An AI agent's name, which is its handle where people have an email.
/// It never contains an @, so it can never match a person's email.
fn validate_agent_name(value: &str) -> Result<String, HttpError> {
  let name = value.trim();
  let valid = (2..=64).contains(&name.len())
    && name
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
  if !valid {
    return Err(HttpError::validation(std::collections::BTreeMap::from([(
      "AGENT_NAME_INVALID".into(),
      "Use 2 to 64 letters, digits, dots, dashes, or underscores.".into(),
    )])));
  }
  Ok(name.to_owned())
}

fn agents_stay_agents() -> HttpError {
  HttpError::bad_request(
    "ROLE_INVALID",
    "An AI agent can't become a person, or a person an AI agent. Create a new account instead.",
  )
}

pub(crate) fn exceeds_access() -> HttpError {
  HttpError::forbidden(
    "ACCESS_EXCEEDS_YOURS",
    "This account has access you don't have, so you can't manage it.",
  )
}

/// The caller's own access and id.
fn caller(identity: &AuthIdentity) -> Result<(&str, &Access), HttpError> {
  match identity {
    AuthIdentity::Admin {
      admin_id, access, ..
    } => Ok((admin_id, access)),
    _ => Err(HttpError::internal()),
  }
}

/// Only roles within the caller's own capabilities and ceiling can be
/// handed out.
async fn require_assignable(
  state: &AppState,
  identity: &AuthIdentity,
  role_id: &str,
) -> Result<(), HttpError> {
  let (_, access) = caller(identity)?;
  let mut conn = state.db.pool().acquire().await?;
  let role = authx::role_access(&mut conn, role_id)
    .await?
    .ok_or_else(|| HttpError::bad_request("ROLE_INVALID", "The role was not found."))?;
  if !access.includes(&role) {
    return Err(HttpError::forbidden(
      "ROLE_EXCEEDS_YOURS",
      "You can only give roles that are within your own.",
    ));
  }
  Ok(())
}

/// A person can manage another account only when that account's role and
/// data access are within their own. Root manages everyone.
pub(crate) async fn require_within(
  state: &AppState,
  identity: &AuthIdentity,
  target_id: &str,
) -> Result<(), HttpError> {
  let (actor_id, access) = caller(identity)?;
  if access.is_root {
    return Ok(());
  }
  let mut conn = state.db.pool().acquire().await?;
  // A disabled account keeps its role, so judge it by that: otherwise
  // anyone could re-enable an account that reaches further than they do.
  let target = authx::account_access(&mut conn, target_id)
    .await?
    .unwrap_or_else(Access::none);
  if !access.includes(&target) {
    return Err(exceeds_access());
  }
  let mine = authx::account_data_access(&mut conn, actor_id, access).await?;
  let theirs = authx::account_data_access(&mut conn, target_id, &target).await?;
  let catalog = Catalog::load(&mut conn).await?;
  if !mine.covers(&theirs, &catalog.projects, &catalog.environments()) {
    return Err(exceeds_access());
  }
  Ok(())
}

/// Signed in to the Admin UI with at least one of `capabilities`. Used
/// before an account is looked up, so someone who may do none of it learns
/// nothing about the account.
fn require_any_browser<'a>(
  identity: &'a AuthIdentity,
  capabilities: &[Capability],
) -> Result<(&'a str, &'a Access), HttpError> {
  require_browser(identity)?;
  let (actor_id, access) = caller(identity)?;
  if !capabilities
    .iter()
    .any(|capability| access.has(*capability))
  {
    return Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your role does not allow this operation.",
    ));
  }
  Ok((actor_id, access))
}

/// What touching someone's account needs, before it is looked up: people
/// are managed with users.manage, AI agents by their owner or agents.manage.
const ACCOUNT_CHANGERS: [Capability; 3] = [
  Capability::UsersManage,
  Capability::AgentsView,
  Capability::AgentsManage,
];

/// Who may change an AI agent: its owner while their role holds
/// agents.view, or someone with agents.manage whose access covers it. An
/// owner needs no more, because an agent never reaches past its owner.
async fn require_agent_control(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  owner_id: Option<&str>,
) -> Result<(), HttpError> {
  let (actor_id, access) = caller(identity)?;
  if access.has(Capability::AgentsView) && owner_id == Some(actor_id) {
    return Ok(());
  }
  require_capability_browser(identity, Capability::AgentsManage)?;
  require_within(state, identity, id).await
}

/// The account someone means to change. Without users.manage only AI
/// agents can be theirs to change, so a person, root, and a missing id all
/// get the same refusal and ids can't be probed.
async fn changeable(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<Account, HttpError> {
  let (_, access) = caller(identity)?;
  let manages_people = access.has(Capability::UsersManage);
  match repository::account(state.db.pool(), id).await? {
    Some(account) if account.agent || manages_people => Ok(account),
    _ if !manages_people => Err(HttpError::forbidden(
      AUTHORIZATION_DENIED,
      "Your role does not allow this operation.",
    )),
    _ => Err(user_not_found()),
  }
}

/// Deleting, disabling, or enabling an account: a person needs
/// users.manage, an AI agent its owner or agents.manage. Either way the
/// account must be within the caller's reach. An owner re-enables their
/// agent only when they disabled it: one turned off with agents.manage
/// stays off until agents.manage turns it back on.
async fn require_account_change(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
  account: &Account,
  enabling: bool,
) -> Result<(), HttpError> {
  if !account.agent {
    require_capability_browser(identity, Capability::UsersManage)?;
    return require_within(state, identity, id).await;
  }
  let mut owner = account.owner_id.as_deref();
  if enabling && account.status == "disabled" && owner.is_some() {
    let disabled_by = repository::last_disabled_by(state.db.pool(), id).await?;
    if disabled_by.as_deref() != owner {
      owner = None;
    }
  }
  require_agent_control(state, identity, id, owner).await
}

/// Whether the caller may see an account: people with users.view, every AI
/// agent with agents.view_all, and the agents they own with agents.view.
/// agents.manage also sees the people who could own an agent, so it can
/// hand one over without seeing everyone.
fn sees(
  access: &Access,
  actor_id: &str,
  row: &UserRow,
) -> bool {
  if row.agent {
    access.has(Capability::AgentsViewAll)
      || (access.has(Capability::AgentsView) && row.owner_id.as_deref() == Some(actor_id))
  } else {
    access.has(Capability::UsersView) || (access.has(Capability::AgentsManage) && row.can_own)
  }
}

/// What listing or reading accounts needs: any one of these.
const ACCOUNT_VIEWERS: [Capability; 4] = [
  Capability::UsersView,
  Capability::AgentsView,
  Capability::AgentsViewAll,
  Capability::AgentsManage,
];

/// What to do once the instance holds as many active users as the limit
/// allows. The Status check reuses it.
pub(crate) fn user_limit_message(max: u32) -> String {
  let users = if max == 1 { "user" } else { "users" };
  format!(
    "This instance allows up to {max} active {users}. To add or enable a user, disable or delete an active user, or ask your instance administrator to increase the limit."
  )
}

/// Refused when the instance already holds as many active users as the
/// limit allows.
fn user_limit_reached(max: u32) -> HttpError {
  HttpError::conflict(USER_LIMIT_REACHED, &user_limit_message(max))
}

/// Refuse early, before slower work like hashing a password, when the
/// people limit is already reached. The write checks again.
async fn require_room(
  state: &AppState,
  limit: Option<u32>,
) -> Result<(), HttpError> {
  if let Some(max) = limit
    && repository::active_people(state.db.pool()).await? >= i64::from(max)
  {
    return Err(user_limit_reached(max));
  }
  Ok(())
}

fn user_not_found() -> HttpError {
  HttpError::not_found("USER_NOT_FOUND", "The user was not found.")
}

/// The all-projects grant a new or changed account starts with. Only root
/// gives it, and only to admins; everyone else starts with no data access
/// and gets grants per project.
fn starting_grant(
  identity: &AuthIdentity,
  role_id: &str,
) -> Option<&'static str> {
  let (_, access) = caller(identity).ok()?;
  (role_id == "rol_admin" && access.has(Capability::GrantsAllProjects)).then_some("project_manager")
}

fn parse_role(value: &str) -> Result<AdminRole, HttpError> {
  AdminRole::parse(value).ok_or_else(HttpError::internal)
}

fn user(
  row: UserRow,
  groups: Vec<GroupRef>,
) -> Result<UserResponse, HttpError> {
  Ok(UserResponse {
    role: parse_role(&row.role)?,
    status: UserStatus::parse(&row.status).ok_or_else(HttpError::internal)?,
    last_sign_in_at: row.last_sign_in_at,
    groups,
    id: row.id,
    email: row.email,
    role_id: row.role_id,
    role_name: row.role_name,
    created_at: row.created_at,
    updated_at: row.updated_at,
    has_password: row.has_password,
    sso_linked: row.sso_linked,
    owner_id: row.owner_id,
    owner_email: row.owner_email,
  })
}

async fn fetch_user(
  state: &AppState,
  id: &str,
) -> Result<UserResponse, HttpError> {
  let row = repository::find(state.db.pool(), id)
    .await?
    .ok_or_else(|| HttpError::not_found("USER_NOT_FOUND", "The user was not found."))?;
  let groups = grants::repository::groups_of(state.db.pool(), id).await?;
  user(row, groups)
}

pub async fn list(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<UserResponse>, HttpError> {
  let (actor_id, access) = require_any_browser(identity, &ACCOUNT_VIEWERS)?;
  let mut groups: HashMap<String, Vec<GroupRef>> = HashMap::new();
  for (user_id, group) in repository::memberships(state.db.pool()).await? {
    groups.entry(user_id).or_default().push(group);
  }
  repository::list(state.db.pool())
    .await?
    .into_iter()
    .filter(|row| sees(access, actor_id, row))
    .map(|row| {
      let mine = groups.remove(&row.id).unwrap_or_default();
      user(row, mine)
    })
    .collect()
}

pub async fn identities(
  state: &AppState,
  identity: &AuthIdentity,
) -> Result<Vec<UserIdentity>, HttpError> {
  require_capability_browser(identity, Capability::UsersView)?;
  Ok(
    repository::identities(state.db.pool())
      .await?
      .into_iter()
      .map(|(id, email)| UserIdentity { id, email })
      .collect(),
  )
}

pub async fn get(
  state: &AppState,
  identity: &AuthIdentity,
  id: &str,
) -> Result<UserResponse, HttpError> {
  let (actor_id, access) = require_any_browser(identity, &ACCOUNT_VIEWERS)?;
  // An account the caller may not see doesn't exist for them.
  let row = repository::find(state.db.pool(), id)
    .await?
    .filter(|row| sees(access, actor_id, row))
    .ok_or_else(user_not_found)?;
  let groups = grants::repository::groups_of(state.db.pool(), id).await?;
  user(row, groups)
}

pub async fn create(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  request: CreateUserRequest,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_any_browser(identity, &[Capability::UsersManage, Capability::AgentsView])?;
  let chosen = choose_role(state, request.role, request.role_id.as_deref()).await?;
  // People who already hold a custom role keep it, but nobody new gets one
  // while custom roles are off.
  if !chosen.builtin {
    state.config.features.require(Feature::RbacRoles)?;
  }
  let agent = chosen.token_only;
  // Whoever creates an AI agent owns it, so it takes agents.view, which an
  // owner needs to run it; agents.manage alone doesn't. People are added
  // with users.manage.
  require_capability_browser(
    identity,
    if agent {
      Capability::AgentsView
    } else {
      Capability::UsersManage
    },
  )?;
  // Without RBAC nobody adds people; AI agents still work.
  if !agent {
    state.config.features.require(Feature::Rbac)?;
  }
  // Only people count toward the limit, never AI agents.
  let limit = if agent {
    None
  } else {
    state.config.features.user_limit()
  };
  require_room(state, limit).await?;
  let email = if agent {
    validate_agent_name(&request.email)?
  } else {
    common::validate_email(&request.email)?
  };
  let password = request.password.filter(|password| !password.is_empty());
  let has_password = password.is_some();
  let hash = match password {
    Some(_) if agent => {
      return Err(HttpError::bad_request(
        "PASSWORD_NOT_ALLOWED",
        "AI agents sign in with tokens only, so they have no password.",
      ));
    }
    // An AI agent never signs in, so it never needs a password.
    None if agent => common::NO_PASSWORD_HASH.to_owned(),
    Some(password) => {
      common::validate_password(&password)?;
      common::hash_password_async(password).await?
    }
    // Without a password the only way in is single sign-on, so it must be on.
    None
      if crate::modules::sso::service::policy(state)
        .await?
        .enabled_name
        .is_some() =>
    {
      common::NO_PASSWORD_HASH.to_owned()
    }
    None => {
      return Err(HttpError::validation(std::collections::BTreeMap::from([(
        PASSWORD_REQUIRED.into(),
        "Enter a password. Accounts without one need single sign-on turned on.".into(),
      )])));
    }
  };
  let id = token::public_id("usr_");
  let now = Utc::now().to_rfc3339();
  let (actor_id, actor_email) = actor(identity);
  require_assignable(state, identity, &chosen.id).await?;
  let mut tx = state.db.pool().begin().await?;
  let inserted = repository::insert(
    &mut tx,
    NewUser {
      id: &id,
      email: &email,
      password_hash: &hash,
      role: chosen.legacy.as_str(),
      role_id: &chosen.id,
      // The person who creates an AI agent answers for it.
      owner_id: agent.then_some(actor_id),
      created_by: actor_id,
      now: &now,
      limit,
    },
  )
  .await;
  match inserted {
    Ok(true) => {}
    Ok(false) => return Err(user_limit_reached(limit.unwrap_or_default())),
    Err(error) if error.to_string().contains("UNIQUE") => {
      return Err(HttpError::conflict(
        "EMAIL_ALREADY_EXISTS",
        "An account with this email already exists.",
      ));
    }
    Err(error) => return Err(error.into()),
  }
  if let Some(level) = starting_grant(identity, &chosen.id) {
    repository::set_all_projects_grant(&mut tx, &id, Some(level), actor_id, &now).await?;
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "admin.created",
    None,
    None,
    Some("admin"),
    Some(&id),
    serde_json::json!({"email":email,"role":chosen.id,"password":has_password}),
  )
  .await?;
  tx.commit().await?;
  fetch_user(state, &id).await
}

pub async fn update(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  request: UpdateUserRequest,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_any_browser(identity, &ACCOUNT_CHANGERS)?;
  let account = changeable(state, identity, id).await?;
  let Account {
    email: old_email,
    is_root,
    role_id: old_role_id,
    agent,
    owner_id: account_owner,
    ..
  } = account;
  let (actor_id, _) = actor(identity);
  let changes_role = request.role.is_some() || request.role_id.is_some();
  if is_root && id != actor_id {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  if id == actor_id && changes_role {
    return Err(HttpError::forbidden(
      "SELF_ROLE_CHANGE",
      "You cannot change your own role.",
    ));
  }
  if is_root && changes_role {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root role cannot be changed.",
    ));
  }
  if is_root && request.password.is_some() {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "Use the root password change flow to change the root password.",
    ));
  }
  // Who may change it comes before what the request asks, so an account
  // the caller can't change tells them nothing more.
  if agent && (changes_role || request.owner_id.is_some()) {
    // Handing an agent to someone else, or changing what it is, is for
    // agents.manage, never only its owner.
    require_capability_browser(identity, Capability::AgentsManage)?;
    require_within(state, identity, id).await?;
  } else if agent {
    require_agent_control(state, identity, id, account_owner.as_deref()).await?;
  } else if !is_root {
    require_within(state, identity, id).await?;
  }
  // Without RBAC people keep their email and role. Their own details, a new
  // password someone sets for them, and AI agents still change, so nobody
  // is locked out for good.
  if !agent && id != actor_id && (changes_role || request.email.is_some()) {
    state.config.features.require(Feature::Rbac)?;
  }
  let new_role = if changes_role {
    Some(choose_role(state, request.role, request.role_id.as_deref()).await?)
  } else {
    None
  };
  if new_role
    .as_ref()
    .is_some_and(|role| role.token_only != agent)
  {
    return Err(agents_stay_agents());
  }
  if agent && request.password.is_some() {
    return Err(HttpError::bad_request(
      "PASSWORD_NOT_ALLOWED",
      "AI agents sign in with tokens only, so they have no password.",
    ));
  }
  if request.owner_id.is_some() && !agent {
    return Err(HttpError::bad_request(
      "OWNER_INVALID",
      "Only AI agents have an owner.",
    ));
  }
  // Keeping the same role changes nothing.
  let new_role = new_role.filter(|role| old_role_id.as_deref() != Some(role.id.as_str()));
  if new_role.as_ref().is_some_and(|role| !role.builtin) {
    state.config.features.require(Feature::RbacRoles)?;
  }
  if let Some(role) = &new_role {
    require_assignable(state, identity, &role.id).await?;
  }
  let email = match request.email {
    Some(value) if agent => validate_agent_name(&value)?,
    Some(value) => common::validate_email(&value)?,
    None => old_email,
  };
  let password_hash = if let Some(password) = request.password {
    common::validate_password(&password)?;
    Some(common::hash_password_async(password).await?)
  } else {
    None
  };
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let updated = repository::update(
    &mut tx,
    id,
    &email,
    password_hash.as_deref(),
    new_role
      .as_ref()
      .map(|role| (role.legacy.as_str(), role.id.as_str())),
    &now,
  )
  .await;
  // A new role starts over on all projects; project grants stay and are
  // capped by the new role's ceiling.
  let result = match (updated, &new_role) {
    (Ok(()), Some(role)) => {
      repository::set_all_projects_grant(
        &mut tx,
        id,
        starting_grant(identity, &role.id),
        actor_id,
        &now,
      )
      .await
    }
    (other, _) => other,
  };
  if let Err(error) = result {
    if error.to_string().contains("UNIQUE") {
      return Err(HttpError::conflict(
        "EMAIL_ALREADY_EXISTS",
        "An account with this email already exists.",
      ));
    }
    return Err(error.into());
  }
  if let Some(owner_id) = &request.owner_id {
    if !repository::can_own(&mut tx, owner_id).await? {
      return Err(HttpError::bad_request(
        "OWNER_INVALID",
        "The owner must be an active person who can create AI agents.",
      ));
    }
    repository::set_owner(&mut tx, id, owner_id).await?;
  }
  // The old owner may still hold the agent's tokens. A new owner starts
  // with none, so nobody keeps acting through access they no longer own.
  let revoked = match &request.owner_id {
    Some(owner_id) if account_owner.as_deref() != Some(owner_id.as_str()) => {
      tokens::personal::repository::revoke_all(&mut tx, id, &now).await?
    }
    _ => Vec::new(),
  };
  repository::revoke_sessions(&mut tx, id, &now).await?;
  let (_, actor_email) = actor(identity);
  for (token_id, name) in &revoked {
    common::audit(
      &mut tx,
      identity.audit_actor_type(),
      Some(actor_id),
      Some(actor_email),
      "personal_token.revoked",
      None,
      None,
      Some("personal_token"),
      Some(token_id),
      serde_json::json!({"userId": id, "user": email, "name": name, "reason": "owner_changed"}),
    )
    .await?;
  }
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "admin.updated",
    None,
    None,
    Some("admin"),
    Some(id),
    serde_json::json!({"email":email,"role":new_role.as_ref().map(|role| &role.id),"ownerId":request.owner_id}),
  )
  .await?;
  tx.commit().await?;
  fetch_user(state, id).await
}

pub async fn delete(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<(), HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_any_browser(identity, &ACCOUNT_CHANGERS)?;
  let (actor_id, actor_email) = actor(identity);
  if id == actor_id {
    return Err(HttpError::forbidden(
      "SELF_DELETE_FORBIDDEN",
      "You cannot delete your own account.",
    ));
  }
  let account = changeable(state, identity, id).await?;
  if account.is_root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be deleted.",
    ));
  }
  require_account_change(state, identity, id, &account, false).await?;
  if repository::owned_count(state.db.pool(), id).await? > 0 {
    return Err(HttpError::conflict(
      "OWNS_AI_AGENTS",
      "This person owns AI agents. Delete them or give them another owner first.",
    ));
  }
  let mut tx = state.db.pool().begin().await?;
  repository::delete(&mut tx, id).await?;
  common::audit(
    &mut tx,
    identity.audit_actor_type(),
    Some(actor_id),
    Some(actor_email),
    "admin.deleted",
    None,
    None,
    Some("admin"),
    Some(id),
    serde_json::json!({}),
  )
  .await?;
  tx.commit().await?;
  Ok(())
}

/// Remove a user's single sign-on link, for example after their identity
/// was recreated at the provider. A verified email links them again on the
/// next sign-in. Root's link can only be removed by root.
pub async fn unlink_sso(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_capability_browser(identity, Capability::UsersManage)?;
  let target = fetch_user(state, id).await?;
  let (actor_id, _) = actor(identity);
  // Only root changes root.
  if target.role == AdminRole::Root && id != actor_id {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  if target.role != AdminRole::Root {
    require_within(state, identity, id).await?;
  }
  crate::modules::sso::service::unlink(state, identity, id, &target.email).await?;
  fetch_user(state, id).await
}

/// Disable an account, or enable it again. A disabled account can't sign
/// in, its sessions end, and its tokens are refused until it is enabled.
/// Asking for the status it already has changes nothing.
pub async fn set_status(
  state: &AppState,
  identity: &AuthIdentity,
  headers: &HeaderMap,
  id: &str,
  status: UserStatus,
) -> Result<UserResponse, HttpError> {
  require_mutation(identity, headers)?;
  require_recent_browser_auth(identity)?;
  require_any_browser(identity, &ACCOUNT_CHANGERS)?;
  let (actor_id, actor_email) = actor(identity);
  let disable = status == UserStatus::Disabled;
  if id == actor_id {
    return Err(HttpError::forbidden(
      "SELF_DISABLE_FORBIDDEN",
      if disable {
        "You cannot disable your own account."
      } else {
        "You cannot enable your own account."
      },
    ));
  }
  let account = changeable(state, identity, id).await?;
  if account.is_root {
    return Err(HttpError::forbidden(
      "ROOT_PROTECTED",
      "The root administrator cannot be modified.",
    ));
  }
  require_account_change(state, identity, id, &account, !disable).await?;
  let email = account.email;
  let (value, action) = if disable {
    ("disabled", "user.disabled")
  } else {
    ("active", "user.enabled")
  };
  // Enabling a person takes a spot under the people limit; AI agents and
  // disabling never do.
  let limit = if disable || account.agent || account.status != "disabled" {
    None
  } else {
    state.config.features.user_limit()
  };
  require_room(state, limit).await?;
  let now = Utc::now().to_rfc3339();
  let mut tx = state.db.pool().begin().await?;
  let changed = repository::set_status(&mut tx, id, value, limit, &now).await?;
  // Someone else may have enabled the account in the meantime; otherwise
  // the limit stopped it.
  if !changed
    && let Some(max) = limit
    && repository::status(&mut tx, id).await?.as_deref() == Some("disabled")
  {
    return Err(user_limit_reached(max));
  }
  if changed {
    if disable {
      repository::revoke_sessions(&mut tx, id, &now).await?;
      repository::cancel_device_approvals(&mut tx, id, &now).await?;
    }
    common::audit(
      &mut tx,
      identity.audit_actor_type(),
      Some(actor_id),
      Some(actor_email),
      action,
      None,
      None,
      Some("admin"),
      Some(id),
      serde_json::json!({"email":email}),
    )
    .await?;
  }
  tx.commit().await?;
  fetch_user(state, id).await
}
