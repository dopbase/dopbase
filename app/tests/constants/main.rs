use app::constants::api;
use std::{fs, path::Path};

#[test]
fn route_templates_match_the_public_contract() {
  let actual = [
    api::health::ROOT,
    api::bootstrap::STATUS,
    api::bootstrap::ADMIN,
    api::bootstrap::RESTORE,
    api::auth::LOGIN,
    api::auth::LOGOUT,
    api::auth::SESSION,
    api::auth::REAUTHENTICATE,
    api::auth::CHANGE_PASSWORD,
    api::projects::COLLECTION,
    api::projects::INIT,
    api::projects::ITEM,
    api::projects::ENVIRONMENTS,
    api::environments::COLLECTION,
    api::environments::RESOLVE,
    api::environments::ITEM,
    api::secrets::COLLECTION,
    api::secrets::ITEM,
    api::secrets::REVEAL,
    api::secrets::IMPORT,
    api::secrets::LAYOUT,
    api::secrets::EXPORT,
    api::secrets::RUNTIME,
    api::tokens::COLLECTION,
    api::tokens::REVOKE,
    api::audit::COLLECTION,
    api::instance::ROOT,
    api::instance::PUBLIC_STATUS,
    api::instance::FACTORY_RESET,
    api::users::COLLECTION,
    api::users::ITEM,
    api::service_accounts::COLLECTION,
    api::service_accounts::ITEM,
    api::service_accounts::TOKENS,
    api::service_accounts::REVOKE_TOKEN,
    api::backups::COLLECTION,
    api::backups::MASTER_KEY,
    api::backups::UPLOAD,
    api::backups::ITEM,
    api::backups::RESTORE,
  ];
  let expected = [
    "/api/v1/health",
    "/api/v1/bootstrap/status",
    "/api/v1/bootstrap/admin",
    "/api/v1/bootstrap/restore",
    "/api/v1/auth/login",
    "/api/v1/auth/logout",
    "/api/v1/auth/session",
    "/api/v1/auth/reauthenticate",
    "/api/v1/auth/change-password",
    "/api/v1/projects",
    "/api/v1/projects/init",
    "/api/v1/projects/{project_ref}",
    "/api/v1/projects/{project_ref}/environments",
    "/api/v1/environments",
    "/api/v1/environments/resolve",
    "/api/v1/environments/{environment_id}",
    "/api/v1/environments/{environment_id}/secrets",
    "/api/v1/environments/{environment_id}/secrets/{key}",
    "/api/v1/environments/{environment_id}/secrets/{key}/reveal",
    "/api/v1/environments/{environment_id}/secrets/import",
    "/api/v1/environments/{environment_id}/secrets/layout",
    "/api/v1/environments/{environment_id}/secrets/export",
    "/api/v1/environments/{environment_id}/secrets/runtime",
    "/api/v1/environments/{environment_id}/tokens",
    "/api/v1/tokens/{token_id}/revoke",
    "/api/v1/audit-events",
    "/api/v1/instance",
    "/api/v1/status",
    "/api/v1/instance/factory-reset",
    "/api/v1/users",
    "/api/v1/users/{id}",
    "/api/v1/service-accounts",
    "/api/v1/service-accounts/{id}",
    "/api/v1/service-accounts/{id}/tokens",
    "/api/v1/service-accounts/{id}/tokens/{token_id}/revoke",
    "/api/v1/backups",
    "/api/v1/backups/master-key",
    "/api/v1/backups/upload",
    "/api/v1/backups/{key}",
    "/api/v1/backups/{key}/restore",
  ];

  assert_eq!(actual, expected);
  assert_eq!(api::PREFIX, "/api/");
  assert_eq!(api::docs::UI, "/api/docs");
  assert_eq!(api::docs::OPENAPI, "/api/v1/openapi.json");
}

#[test]
fn path_builders_match_the_previous_format_output() {
  assert_eq!(api::projects::item("payments"), "/api/v1/projects/payments");
  assert_eq!(
    api::projects::environments("payments"),
    "/api/v1/projects/payments/environments"
  );
  assert_eq!(
    api::environments::item("env_01"),
    "/api/v1/environments/env_01"
  );
  assert_eq!(
    api::secrets::collection("env_01"),
    "/api/v1/environments/env_01/secrets"
  );
  assert_eq!(
    api::secrets::item("env_01", "API_KEY"),
    "/api/v1/environments/env_01/secrets/API_KEY"
  );
  assert_eq!(
    api::secrets::reveal("env_01", "API_KEY"),
    "/api/v1/environments/env_01/secrets/API_KEY/reveal"
  );
  assert_eq!(
    api::secrets::import("env_01"),
    "/api/v1/environments/env_01/secrets/import"
  );
  assert_eq!(
    api::secrets::layout("env_01"),
    "/api/v1/environments/env_01/secrets/layout"
  );
  assert_eq!(
    api::secrets::export("env_01"),
    "/api/v1/environments/env_01/secrets/export"
  );
  assert_eq!(
    api::secrets::runtime("env_01"),
    "/api/v1/environments/env_01/secrets/runtime"
  );
  assert_eq!(
    api::tokens::collection("env_01"),
    "/api/v1/environments/env_01/tokens"
  );
  assert_eq!(
    api::tokens::revoke("tok_01"),
    "/api/v1/tokens/tok_01/revoke"
  );
  assert_eq!(api::users::item("usr_01"), "/api/v1/users/usr_01");
  assert_eq!(
    api::service_accounts::item("svc_01"),
    "/api/v1/service-accounts/svc_01"
  );
  assert_eq!(
    api::service_accounts::tokens("svc_01"),
    "/api/v1/service-accounts/svc_01/tokens"
  );
  assert_eq!(
    api::service_accounts::revoke_token("svc_01", "tok_01"),
    "/api/v1/service-accounts/svc_01/tokens/tok_01/revoke"
  );
  assert_eq!(
    api::backups::item("snapshot.dop"),
    "/api/v1/backups/snapshot.dop"
  );
  assert_eq!(
    api::backups::restore("snapshot.dop"),
    "/api/v1/backups/snapshot.dop/restore"
  );

  // Path parameters remain unencoded to preserve the existing format! behavior.
  assert_eq!(api::projects::item("a/b"), "/api/v1/projects/a/b");
}

#[test]
fn query_builders_keep_the_existing_encoding() {
  assert_eq!(api::environments::list(None), "/api/v1/environments");
  assert_eq!(
    api::environments::list(Some("payment service/main")),
    "/api/v1/environments?project=payment+service%2Fmain"
  );
  assert_eq!(
    api::environments::resolve("payment service/main"),
    "/api/v1/environments/resolve?reference=payment+service%2Fmain"
  );
}

#[test]
fn production_api_literals_live_only_in_the_route_catalog() {
  let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
  let catalog = source_root.join("constants/api.rs");
  let mut directories = vec![source_root];

  while let Some(directory) = directories.pop() {
    for entry in fs::read_dir(directory).unwrap() {
      let path = entry.unwrap().path();
      if path.is_dir() {
        directories.push(path);
      } else if path.extension().is_some_and(|extension| extension == "rs") && path != catalog {
        let source = fs::read_to_string(&path).unwrap();
        assert!(
          !source.contains("\"/api"),
          "hardcoded API path found in {}",
          path.display()
        );
      }
    }
  }
}
