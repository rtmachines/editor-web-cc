-- Validate the two checks that migration 0013 added as NOT VALID (ADR-0156, data-model.md section 3.1).
-- VALIDATE CONSTRAINT takes a SHARE UPDATE EXCLUSIVE lock, so reads and writes continue.

ALTER TABLE machine VALIDATE CONSTRAINT machine_db_number_shape;
ALTER TABLE machine VALIDATE CONSTRAINT machine_db_number_range;
