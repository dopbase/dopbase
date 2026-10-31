-- When the account last signed in interactively: a password, single sign-on,
-- or CLI browser login that issued a session. Confirming a session again and
-- using a token leave it alone. Empty until the first sign-in.
ALTER TABLE admins ADD COLUMN last_sign_in_at TEXT NULL;
