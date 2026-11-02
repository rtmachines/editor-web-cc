-- Role export_client of app_user (data-model.md section 3.5, ADR-0109 (e)).
-- Both constraints are NOT VALID, so the file scans no table under an exclusive lock.
-- Migration 0010 runs VALIDATE CONSTRAINT on both.

ALTER TABLE app_user DROP CONSTRAINT app_user_role_value;
ALTER TABLE app_user ADD CONSTRAINT app_user_role_value CHECK (
    role IN ('administrator', 'process_engineer', 'operator', 'export_client')
) NOT VALID;

-- The account of a transfer client has no one-time password.
ALTER TABLE app_user ADD CONSTRAINT app_user_export_client_password CHECK (
    role <> 'export_client' OR NOT must_change_password
) NOT VALID;
