ALTER TABLE base_users ADD COLUMN handle TEXT;
UPDATE base_users SET handle = 'user_' || id;
ALTER TABLE base_users ALTER COLUMN handle SET NOT NULL;
ALTER TABLE base_users ADD CONSTRAINT base_users_handle_format CHECK (handle ~ '^[a-z][a-z0-9_-]{2,31}$');
ALTER TABLE base_users ADD CONSTRAINT base_users_handle_namespace CHECK (handle !~ '^user_[0-9]+$' OR handle = 'user_' || id);
CREATE UNIQUE INDEX base_users_handle_unique ON base_users (lower(handle));

CREATE FUNCTION assign_user_handle() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.handle IS NULL THEN NEW.handle := 'user_' || NEW.id; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER base_users_assign_handle BEFORE INSERT ON base_users
FOR EACH ROW EXECUTE FUNCTION assign_user_handle();
