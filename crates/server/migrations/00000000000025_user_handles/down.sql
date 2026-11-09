DROP TRIGGER base_users_assign_handle ON base_users;
DROP FUNCTION assign_user_handle();
ALTER TABLE base_users DROP COLUMN handle;
