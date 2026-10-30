use app::services::authx::{
  self, Access, Capability, DataAccess, EnvironmentTarget, Grant, Level, Scope, SignIn,
};
use app::services::db::{DbClient, MIGRATOR};

fn project(
  id: &str,
  level: Level,
) -> Grant {
  Grant::new(Scope::Project(id.into()), level)
}

fn environment(
  id: &str,
  level: Level,
) -> Grant {
  Grant::new(Scope::Environment(id.into()), level)
}

fn all(level: Level) -> Grant {
  Grant::new(Scope::All, level)
}

const PROD: EnvironmentTarget<'static> = EnvironmentTarget {
  project_id: "prj_pay",
  environment_id: "env_prod",
  protected: false,
};

fn protected() -> EnvironmentTarget<'static> {
  EnvironmentTarget {
    protected: true,
    ..PROD
  }
}

#[test]
fn no_grant_gives_no_access() {
  assert_eq!(
    authx::environment_level(&[], PROD, Level::ProjectManager),
    Level::None
  );
  assert_eq!(
    authx::environment_level(
      &[project("prj_other", Level::Write)],
      PROD,
      Level::ProjectManager
    ),
    Level::None
  );
}

#[test]
fn the_most_specific_tier_decides() {
  let grants = [
    all(Level::Write),
    project("prj_pay", Level::Read),
    environment("env_prod", Level::Metadata),
  ];
  assert_eq!(
    authx::environment_level(&grants, PROD, Level::ProjectManager),
    Level::Metadata
  );
  assert_eq!(
    authx::environment_level(&grants[..2], PROD, Level::ProjectManager),
    Level::Read
  );
  assert_eq!(
    authx::environment_level(&grants[..1], PROD, Level::ProjectManager),
    Level::Write
  );
}

#[test]
fn an_environment_grant_of_none_excludes_it_from_a_wider_grant() {
  let grants = [
    project("prj_pay", Level::Write),
    environment("env_prod", Level::None),
  ];
  assert_eq!(
    authx::environment_level(&grants, PROD, Level::ProjectManager),
    Level::None
  );
}

#[test]
fn the_highest_level_within_a_tier_wins() {
  // A direct grant and a group grant on the same project combine upward.
  let grants = [
    project("prj_pay", Level::Metadata),
    project("prj_pay", Level::Write),
  ];
  assert_eq!(
    authx::environment_level(&grants, PROD, Level::ProjectManager),
    Level::Write
  );
}

#[test]
fn a_project_manager_manages_every_environment() {
  let grants = [project("prj_pay", Level::ProjectManager)];
  assert_eq!(
    authx::environment_level(&grants, PROD, Level::ProjectManager),
    Level::Manage
  );
  assert_eq!(
    authx::environment_level(&grants, protected(), Level::ProjectManager),
    Level::Manage
  );
}

#[test]
fn inherited_grants_below_project_manager_stop_at_protected_environments() {
  for grants in [[project("prj_pay", Level::Write)], [all(Level::Manage)]] {
    assert_eq!(
      authx::environment_level(&grants, protected(), Level::ProjectManager),
      Level::None
    );
  }
  let grants = [
    project("prj_pay", Level::Write),
    environment("env_prod", Level::Read),
  ];
  assert_eq!(
    authx::environment_level(&grants, protected(), Level::ProjectManager),
    Level::Read
  );
}

#[test]
fn the_role_ceiling_clamps_every_grant() {
  let grants = [project("prj_pay", Level::ProjectManager)];
  assert_eq!(
    authx::environment_level(&grants, PROD, Level::Metadata),
    Level::Metadata
  );
  assert_eq!(
    authx::project_level(&grants, "prj_pay", Level::Read),
    Level::Read
  );
}

#[test]
fn project_level_ignores_environment_grants() {
  let grants = [environment("env_prod", Level::Manage)];
  assert_eq!(
    authx::project_level(&grants, "prj_pay", Level::ProjectManager),
    Level::None
  );
  let grants = [all(Level::Read), project("prj_pay", Level::ProjectManager)];
  assert_eq!(
    authx::project_level(&grants, "prj_pay", Level::ProjectManager),
    Level::ProjectManager
  );
  assert_eq!(
    authx::project_level(&grants, "prj_other", Level::ProjectManager),
    Level::Read
  );
}

#[test]
fn each_kind_of_principal_reaches_its_own_data() {
  let other = EnvironmentTarget {
    environment_id: "env_dev",
    ..PROD
  };
  assert_eq!(
    DataAccess::Everything.environment(protected()),
    Level::Manage
  );
  assert_eq!(
    DataAccess::Everything.project("prj_any"),
    Level::ProjectManager
  );
  let agent = DataAccess::Uniform(Level::Metadata);
  assert_eq!(agent.environment(protected()), Level::Metadata);
  let runner = DataAccess::Runner("env_prod".into());
  assert_eq!(runner.environment(PROD), Level::Read);
  assert_eq!(runner.environment(other), Level::None);
  assert_eq!(runner.project("prj_pay"), Level::None);
}

#[test]
fn an_environment_grant_shows_its_project_without_rights_over_it() {
  let access = DataAccess::Grants {
    grants: vec![environment("env_prod", Level::Read)],
    ceiling: Level::Manage,
  };
  assert_eq!(access.project("prj_pay"), Level::None);
  assert!(access.sees_project("prj_pay", [PROD].into_iter()));
  assert!(!access.sees_project("prj_other", std::iter::empty()));
}

#[test]
fn covering_another_access_needs_at_least_its_level_everywhere() {
  let manager = DataAccess::Grants {
    grants: vec![project("prj_pay", Level::ProjectManager)],
    ceiling: Level::ProjectManager,
  };
  let writer = DataAccess::Grants {
    grants: vec![project("prj_pay", Level::Write)],
    ceiling: Level::Manage,
  };
  let everywhere = DataAccess::Grants {
    grants: vec![all(Level::Read)],
    ceiling: Level::Manage,
  };
  let projects = ["prj_pay".to_owned(), "prj_other".to_owned()];
  let environments = [
    PROD,
    EnvironmentTarget {
      project_id: "prj_other",
      environment_id: "env_other",
      protected: false,
    },
  ];
  assert!(manager.covers(&writer, &projects, &environments));
  assert!(!writer.covers(&manager, &projects, &environments));
  assert!(!manager.covers(&everywhere, &projects, &environments));
  assert!(DataAccess::Everything.covers(&everywhere, &projects, &environments));
}

#[test]
fn a_role_includes_only_what_it_holds_itself() {
  let admin = Access::role(
    "rol_admin",
    "Admin",
    [Capability::UsersManage, Capability::UsersView],
    Level::ProjectManager,
    SignIn::Interactive,
  );
  let member = Access::role(
    "rol_member",
    "Member",
    [Capability::TokensPersonal],
    Level::Manage,
    SignIn::Interactive,
  );
  let viewer = Access::role(
    "rol_viewer",
    "Viewer",
    [Capability::UsersView],
    Level::Read,
    SignIn::Interactive,
  );
  assert!(admin.includes(&viewer));
  assert!(!admin.includes(&member));
  assert!(!viewer.includes(&admin));
  assert!(!admin.includes(&Access::root()));
  assert!(Access::root().includes(&admin));
}

#[test]
fn levels_and_capabilities_round_trip_their_names() {
  for level in [
    Level::None,
    Level::Metadata,
    Level::Read,
    Level::Write,
    Level::Manage,
    Level::ProjectManager,
  ] {
    assert_eq!(Level::parse(level.as_str()), Some(level));
  }
  for capability in Capability::ALL {
    assert_eq!(Capability::parse(capability.as_str()), Some(capability));
    assert_eq!(
      serde_json::to_value(capability).unwrap(),
      capability.as_str()
    );
  }
  assert_eq!(Capability::parse("users.everything"), None);
}

#[test]
fn a_role_can_never_hold_a_root_only_capability() {
  let access = Access::role(
    "rol_custom",
    "Custom",
    Capability::ALL,
    Level::ProjectManager,
    SignIn::Interactive,
  );
  for capability in Capability::ROOT_ONLY {
    assert!(!access.has(capability), "{capability:?}");
  }
  assert!(access.has(Capability::UsersManage));
  let root = Access::root();
  assert!(
    Capability::ALL
      .into_iter()
      .all(|capability| root.has(capability))
  );
  assert!(!Access::none().has(Capability::UsersView));
}

async fn database() -> DbClient {
  let db = DbClient::connect("sqlite::memory:").await.unwrap();
  MIGRATOR.run(db.pool()).await.unwrap();
  db
}

async fn insert_account(
  db: &DbClient,
  id: &str,
  role: &str,
  role_id: Option<&str>,
  is_root: bool,
) {
  sqlx::query("INSERT INTO admins(id,email,password_hash,role,role_id,is_root,created_at,updated_at) VALUES(?,?,'',?,?,?,'t','t')")
    .bind(id)
    .bind(format!("{id}@example.com"))
    .bind(role)
    .bind(role_id)
    .bind(is_root)
    .execute(db.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn loads_access_from_the_built_in_roles() {
  let db = database().await;
  insert_account(&db, "usr_root", "root", None, true).await;
  insert_account(&db, "usr_admin", "admin", Some("rol_admin"), false).await;
  insert_account(&db, "usr_auditor", "member", Some("rol_auditor"), false).await;
  insert_account(&db, "usr_agent", "member", Some("rol_ai_agent"), false).await;
  insert_account(&db, "usr_orphan", "member", Some("rol_missing"), false).await;
  let mut conn = db.pool().acquire().await.unwrap();

  let root = authx::load_access(&mut conn, "usr_root")
    .await
    .unwrap()
    .unwrap();
  assert_eq!(root, Access::root());

  let admin = authx::load_access(&mut conn, "usr_admin")
    .await
    .unwrap()
    .unwrap();
  assert_eq!(admin.max_level, Level::ProjectManager);
  assert!(admin.has(Capability::UsersManage));
  assert!(admin.has(Capability::AuditRead));
  assert!(!admin.has(Capability::InstanceBackup));
  assert!(!admin.has(Capability::SsoManage));

  let auditor = authx::load_access(&mut conn, "usr_auditor")
    .await
    .unwrap()
    .unwrap();
  assert_eq!(auditor.max_level, Level::Metadata);
  assert!(auditor.has(Capability::AuditVerify));
  assert!(!auditor.has(Capability::UsersManage));

  let agent = authx::load_access(&mut conn, "usr_agent")
    .await
    .unwrap()
    .unwrap();
  assert_eq!(agent.sign_in, SignIn::TokenOnly);
  assert_eq!(agent.max_level, Level::Read);

  // An account whose role is gone authenticates but holds nothing.
  let orphan = authx::load_access(&mut conn, "usr_orphan")
    .await
    .unwrap()
    .unwrap();
  assert_eq!(orphan, Access::none());

  assert!(
    authx::load_access(&mut conn, "usr_nobody")
      .await
      .unwrap()
      .is_none()
  );
  sqlx::query("UPDATE admins SET status='disabled' WHERE id='usr_admin'")
    .execute(&mut *conn)
    .await
    .unwrap();
  assert!(
    authx::load_access(&mut conn, "usr_admin")
      .await
      .unwrap()
      .is_none()
  );
}

#[tokio::test]
async fn resolves_direct_and_group_grants_on_an_environment() {
  let db = database().await;
  insert_account(&db, "usr_member", "member", Some("rol_member"), false).await;
  for statement in [
    "INSERT INTO projects(id,name,created_at,updated_at) VALUES('prj_pay','payments','t','t')",
    "INSERT INTO environments(id,project_id,name,created_at,updated_at) VALUES('env_dev','prj_pay','dev','t','t')",
    "INSERT INTO environments(id,project_id,name,protected,created_at,updated_at) VALUES('env_prod','prj_pay','prod',1,'t','t')",
    "INSERT INTO groups(id,name,created_at,updated_at) VALUES('grp_backend','Backend','t','t')",
    "INSERT INTO group_members(group_id,user_id,created_at) VALUES('grp_backend','usr_member','t')",
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_1','group','grp_backend','project','prj_pay','write','t')",
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_2','user','usr_member','project','prj_pay','metadata','t')",
  ] {
    sqlx::query(statement).execute(db.pool()).await.unwrap();
  }
  let mut conn = db.pool().acquire().await.unwrap();
  let access = authx::load_access(&mut conn, "usr_member")
    .await
    .unwrap()
    .unwrap();

  let level = |environment: &'static str| {
    let access = access.clone();
    let pool = db.pool().clone();
    async move {
      let mut conn = pool.acquire().await.unwrap();
      authx::level_on_environment(&mut conn, "usr_member", &access, environment)
        .await
        .unwrap()
    }
  };
  // The group's write beats the direct metadata grant on the same project.
  assert_eq!(level("env_dev").await, Some(Level::Write));
  // Production is protected, so the project grants stop there.
  assert_eq!(level("env_prod").await, Some(Level::None));
  assert_eq!(level("env_missing").await, None);

  // Leaving the group takes its grant away on the next decision.
  sqlx::query("DELETE FROM group_members")
    .execute(db.pool())
    .await
    .unwrap();
  assert_eq!(level("env_dev").await, Some(Level::Metadata));

  // Root needs no grants.
  assert_eq!(
    authx::level_on_environment(&mut conn, "usr_root", &Access::root(), "env_prod")
      .await
      .unwrap(),
    Some(Level::Manage)
  );
}

#[tokio::test]
async fn grants_disappear_with_what_they_name() {
  let db = database().await;
  insert_account(&db, "usr_member", "member", Some("rol_member"), false).await;
  for statement in [
    "INSERT INTO projects(id,name,created_at,updated_at) VALUES('prj_pay','payments','t','t')",
    "INSERT INTO environments(id,project_id,name,created_at,updated_at) VALUES('env_dev','prj_pay','dev','t','t')",
    "INSERT INTO groups(id,name,created_at,updated_at) VALUES('grp_backend','Backend','t','t')",
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_1','group','grp_backend','all','','read','t')",
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_2','user','usr_member','environment','env_dev','write','t')",
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_3','user','usr_member','project','prj_pay','read','t')",
  ] {
    sqlx::query(statement).execute(db.pool()).await.unwrap();
  }
  let count = || async {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM grants")
      .fetch_one(db.pool())
      .await
      .unwrap()
  };
  sqlx::query("DELETE FROM environments")
    .execute(db.pool())
    .await
    .unwrap();
  assert_eq!(count().await, 2);
  sqlx::query("DELETE FROM projects")
    .execute(db.pool())
    .await
    .unwrap();
  assert_eq!(count().await, 1);
  sqlx::query("DELETE FROM groups")
    .execute(db.pool())
    .await
    .unwrap();
  assert_eq!(count().await, 0);
}

#[tokio::test]
async fn rejects_grants_the_schema_does_not_allow() {
  let db = database().await;
  for statement in [
    // project_manager stops above the environment.
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_a','user','usr_x','environment','env_dev','project_manager','t')",
    // none only excludes an environment.
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_b','user','usr_x','project','prj_pay','none','t')",
    // all-projects grants have no scope id.
    "INSERT INTO grants(id,principal_type,principal_id,scope_type,scope_id,level,created_at) VALUES('grt_c','user','usr_x','all','prj_pay','read','t')",
  ] {
    assert!(
      sqlx::query(statement).execute(db.pool()).await.is_err(),
      "{statement}"
    );
  }
}
