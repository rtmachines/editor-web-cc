-- Validate the two checks that migration 0009 added as NOT VALID (data-model.md section 3.5).
-- VALIDATE CONSTRAINT takes a SHARE UPDATE EXCLUSIVE lock, so reads and writes continue.

ALTER TABLE app_user VALIDATE CONSTRAINT app_user_role_value;
ALTER TABLE app_user VALIDATE CONSTRAINT app_user_export_client_password;
