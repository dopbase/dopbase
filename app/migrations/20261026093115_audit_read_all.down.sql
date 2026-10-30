DELETE FROM role_capabilities WHERE capability = 'audit.read_all';
DELETE FROM role_capabilities WHERE role_id = 'rol_auditor' AND capability = 'tokens.personal';
