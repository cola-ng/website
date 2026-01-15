CREATE TABLE IF NOT EXISTS users (
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

CREATE TABLE user_passwords
(
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hash text NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_codes (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  code_hash text NOT NULL UNIQUE,
  redirect_uri text NOT NULL,
  state text NOT NULL,
  expires_at timestamp with time zone NOT NULL,
  used_at timestamp with time zone NULL,
  created_at timestamp with time zone NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS auth_codes_user_id_created_at_idx
  ON auth_codes (user_id, created_at DESC);