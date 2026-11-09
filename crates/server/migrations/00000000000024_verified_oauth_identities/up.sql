-- Legacy subject-only bindings did not prove provider ownership. Retain them for
-- account management, but require a real provider callback before using them.
ALTER TABLE oauth_identities ADD COLUMN verified_at timestamptz NULL;
