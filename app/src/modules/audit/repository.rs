use super::model::AuditEvent;
use sqlx::SqlitePool;

/// Which events a reader may see.
pub enum Visibility<'a> {
  All,
  /// Instance events, events on these projects, and the reader's own.
  Projects {
    actor_id: &'a str,
    /// A JSON array of project ids.
    projects: String,
  },
  /// Only the reader's own actions.
  Own {
    actor_id: &'a str,
  },
}

#[allow(clippy::too_many_arguments)]
pub async fn list(
  pool: &SqlitePool,
  visibility: &Visibility<'_>,
  before: Option<(&str, &str)>,
  limit: u32,
  action: Option<&str>,
  project: Option<&str>,
  environment: Option<&str>,
  actor: Option<&str>,
) -> Result<Vec<AuditEvent>, sqlx::Error> {
  let (time, id) = before.unwrap_or(("9999-12-31T23:59:59Z", "~"));
  let (mode, reader, projects) = match visibility {
    Visibility::All => ("all", "", "[]".to_owned()),
    Visibility::Projects { actor_id, projects } => ("projects", *actor_id, projects.clone()),
    Visibility::Own { actor_id } => ("own", *actor_id, "[]".to_owned()),
  };
  sqlx::query_as("SELECT id,actor_type,actor_id,actor_label,action,project_id,environment_id,resource_type,resource_id,ip_address,json(metadata) AS metadata,created_at,seq,prev_hash,record_hash FROM audit_events WHERE (created_at < ? OR (created_at = ? AND id < ?)) AND (? IS NULL OR action=?) AND (? IS NULL OR project_id=?) AND (? IS NULL OR environment_id=?) AND (? IS NULL OR actor_id=? OR actor_label=?) AND (? = 'all' OR actor_id = ? OR (? = 'projects' AND (project_id IS NULL OR project_id IN (SELECT value FROM json_each(?))))) ORDER BY created_at DESC,id DESC LIMIT ?")
    .bind(time).bind(time).bind(id).bind(action).bind(action).bind(project).bind(project).bind(environment).bind(environment).bind(actor).bind(actor).bind(actor)
    .bind(mode).bind(reader).bind(mode).bind(projects)
    .bind(limit)
    .fetch_all(pool)
    .await
}
