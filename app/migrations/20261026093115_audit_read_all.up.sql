-- audit.read_all lets a role read every audit event. Without it, audit.read
-- shows instance events and events on the projects the reader reaches, and
-- everyone else sees only their own actions. Auditors also get
-- tokens.personal, so they can hold the audit tokens their role allows.
INSERT INTO role_capabilities (role_id, capability) VALUES
  ('rol_auditor', 'audit.read_all'),
  ('rol_auditor', 'tokens.personal');
