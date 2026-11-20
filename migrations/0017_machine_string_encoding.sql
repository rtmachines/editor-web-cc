-- STRING encoding of a machine (ADR-0202, data-model.md sections 3.1 and 5.2).
-- This is the expand step. Migration 0018 runs VALIDATE CONSTRAINT on both checks.
-- The default is a literal, so ADD COLUMN rewrites no table (ADR-0084 (d)). Each stored machine
-- reads Latin1, so no stored value changes. A previous release that still runs writes no value,
-- and the default gives Latin1. The NOT VALID checks scan no table.

ALTER TABLE machine ADD COLUMN string_encoding TEXT NOT NULL DEFAULT 'Latin1';

ALTER TABLE machine ADD CONSTRAINT machine_string_encoding_value CHECK (string_encoding IN ('Latin1', 'Utf8')) NOT VALID;

-- Utf8 only on CodesysIec and TwinCat3 (ADR-0202 (b)).
ALTER TABLE machine ADD CONSTRAINT machine_string_encoding_platform CHECK (
    string_encoding = 'Latin1' OR plc_platform IN ('CodesysIec', 'TwinCat3')
) NOT VALID;

GRANT UPDATE (string_encoding) ON machine TO recipe_app;
