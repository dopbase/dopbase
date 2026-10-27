INSERT INTO workspaces (id, name, root_path, created_at, updated_at)
SELECT
  'wsp_default',
  'default',
  '/workspace',
  datetime('now'),
  datetime('now')
WHERE NOT EXISTS (SELECT 1 FROM workspaces);

INSERT INTO project_locations (project_id, workspace_id, relative_path)
SELECT
  p.id,
  'wsp_default',
  p.name
FROM projects p
WHERE NOT EXISTS (
  SELECT 1 FROM project_locations l WHERE l.project_id = p.id
)
AND EXISTS (SELECT 1 FROM workspaces WHERE id = 'wsp_default');
