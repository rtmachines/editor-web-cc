-- Validate the two checks that migration 0017 added as NOT VALID (ADR-0202, data-model.md section 3.1).
-- VALIDATE CONSTRAINT takes a SHARE UPDATE EXCLUSIVE lock, so reads and writes continue.

ALTER TABLE machine VALIDATE CONSTRAINT machine_string_encoding_value;
ALTER TABLE machine VALIDATE CONSTRAINT machine_string_encoding_platform;
