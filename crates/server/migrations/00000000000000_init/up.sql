CREATE TABLE IF NOT EXISTS base_users (
  id BIGSERIAL PRIMARY KEY,
  name text NOT NULL UNIQUE,
  email text NULL,
  phone text NULL,

  avatar text,
  display_name text,
  
  verified_at timestamp with time zone,
  limited_at timestamp with time zone,
  limited_by bigint,
  locked_at timestamp with time zone,
  locked_by bigint,
  disabled_at timestamp with time zone,
  disabled_by bigint,
  inviter_id bigint,
  profile jsonb NOT NULL DEFAULT '{}'::jsonb,
  updated_by bigint,
  updated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_by bigint,
  created_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE base_passwords
(
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    hash text NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_codes (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  code_hash text NOT NULL UNIQUE,
  redirect_uri text NOT NULL,
  state text NOT NULL,
  expires_at timestamp with time zone NOT NULL,
  used_at timestamp with time zone NULL,
  created_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS auth_codes_user_id_created_at_idx
  ON auth_codes (user_id, created_at DESC);


drop table if exists base_roles;
CREATE TABLE IF NOT EXISTS base_roles (
  id BIGSERIAL PRIMARY KEY,
  code text NOT NULL UNIQUE,
  name text NOT NULL,
  kind text NOT NULL DEFAULT 'custom'::character varying,
  owner_id bigint NOT NULL,
  description text,
  updated_by bigint,
  updated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_by bigint,
  created_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP
);

drop table if exists base_role_users;
CREATE TABLE IF NOT EXISTS base_role_users (
  role_id BIGINT NOT NULL,
  user_id BIGINT NOT NULL,
  created_at timestamp with time zone NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, role_id)
);

CREATE TABLE IF NOT EXISTS base_role_permissions (
  id BIGSERIAL PRIMARY KEY,
  role_id BIGINT NOT NULL,
  operation text NOT NULL,
  created_at timestamp with time zone NOT NULL DEFAULT now(),
  UNIQUE (role_id, operation)
);

CREATE TABLE IF NOT EXISTS oauth_identities (
  id BIGSERIAL PRIMARY KEY,
  provider text NOT NULL,
  provider_user_id text NOT NULL,
  email text NULL,
  user_id BIGINT NULL,
  created_at timestamp with time zone NOT NULL DEFAULT now(),
  updated_at timestamp with time zone NOT NULL DEFAULT now(),
  UNIQUE (provider, provider_user_id)
);

CREATE TABLE IF NOT EXISTS oauth_login_sessions (
  id BIGSERIAL PRIMARY KEY,
  provider text NOT NULL,
  state text NOT NULL UNIQUE,
  redirect_uri text NOT NULL,
  created_at timestamp with time zone NOT NULL DEFAULT now(),
  expires_at timestamp with time zone NOT NULL
);

