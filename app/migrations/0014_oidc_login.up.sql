-- Links a provisioned admin account to an identity at an OIDC provider.
-- The pair (issuer, subject) is the stable key: it survives an email change at
-- the provider and prevents a recycled email from inheriting the account.
CREATE TABLE oidc_identities (
  id TEXT PRIMARY KEY,
  admin_id TEXT NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
  issuer TEXT NOT NULL,
  subject TEXT NOT NULL,
  email TEXT,
  created_at TEXT NOT NULL,
  last_login_at TEXT NOT NULL
);

CREATE UNIQUE INDEX oidc_identities_subject_idx ON oidc_identities(issuer, subject);
CREATE INDEX oidc_identities_admin_idx ON oidc_identities(admin_id);

-- Short-lived browser sign-in attempts. A row holds the PKCE verifier and the
-- nonce until the provider redirects back; `state` is the single-use key.
CREATE TABLE oidc_login_flows (
  state TEXT PRIMARY KEY,
  nonce TEXT NOT NULL,
  code_verifier TEXT NOT NULL,
  redirect_to TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX oidc_login_flows_created_idx ON oidc_login_flows(created_at);
