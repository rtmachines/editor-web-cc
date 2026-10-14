-- Seed of the global primitive types (data-model.md section 7, Assumption A3).
-- Identifiers follow design D11: 5eed0000-0000-4000-8000-0000000000NN, where NN is the row
-- number of the matrix in hexadecimal. The 80-character rows of STRING and WSTRING take 1e and 1f.
-- A shipped migration never changes. A later migration adds or removes a row (section 7).

INSERT INTO plc_type (id, kind, name, platforms, size_bytes, string_length, leaf_class) VALUES
    ('5eed0000-0000-4000-8000-000000000001', 'primitive', 'BOOL', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 1, NULL, 'bool'),
    ('5eed0000-0000-4000-8000-000000000002', 'primitive', 'BYTE', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 1, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000003', 'primitive', 'WORD', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 2, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000004', 'primitive', 'DWORD', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 4, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000005', 'primitive', 'LWORD', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500'], 8, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000006', 'primitive', 'SINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500'], 1, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000007', 'primitive', 'USINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500'], 1, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000008', 'primitive', 'INT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 2, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000009', 'primitive', 'UINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500'], 2, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-00000000000a', 'primitive', 'DINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 4, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-00000000000b', 'primitive', 'UDINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500'], 4, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-00000000000c', 'primitive', 'LINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500'], 8, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-00000000000d', 'primitive', 'ULINT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500'], 8, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-00000000000e', 'primitive', 'REAL', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 4, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-00000000000f', 'primitive', 'LREAL', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500'], 8, NULL, 'numeric'),
    ('5eed0000-0000-4000-8000-000000000010', 'primitive', 'STRING', ARRAY['S7_1200', 'S7_1500', 'S7_300'], 256, 254, 'string'),
    ('5eed0000-0000-4000-8000-000000000011', 'primitive', 'WSTRING', ARRAY['S7_1200', 'S7_1500'], 512, 254, 'string'),
    ('5eed0000-0000-4000-8000-000000000012', 'primitive', 'CHAR', ARRAY['S7_1200', 'S7_1500', 'S7_300'], 1, NULL, 'char'),
    ('5eed0000-0000-4000-8000-000000000013', 'primitive', 'WCHAR', ARRAY['S7_1200', 'S7_1500'], 2, NULL, 'char'),
    ('5eed0000-0000-4000-8000-000000000014', 'primitive', 'TIME', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 4, NULL, 'duration'),
    ('5eed0000-0000-4000-8000-000000000015', 'primitive', 'LTIME', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500'], 8, NULL, 'duration'),
    ('5eed0000-0000-4000-8000-000000000016', 'primitive', 'DATE', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 2, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-000000000017', 'primitive', 'LDATE', ARRAY['CodesysIec', 'TwinCat3'], 8, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-000000000018', 'primitive', 'TIME_OF_DAY', ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300'], 4, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-000000000019', 'primitive', 'LTOD', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500'], 8, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-00000000001a', 'primitive', 'DATE_AND_TIME', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500', 'S7_300'], 8, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-00000000001b', 'primitive', 'LDT', ARRAY['CodesysIec', 'TwinCat3', 'S7_1500'], 8, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-00000000001c', 'primitive', 'DTL', ARRAY['S7_1200', 'S7_1500'], 12, NULL, 'instant'),
    ('5eed0000-0000-4000-8000-00000000001d', 'primitive', 'S5TIME', ARRAY['S7_1500', 'S7_300'], 2, NULL, 'duration'),
    ('5eed0000-0000-4000-8000-00000000001e', 'primitive', 'STRING', ARRAY['CodesysIec', 'TwinCat3'], 82, 80, 'string'),
    ('5eed0000-0000-4000-8000-00000000001f', 'primitive', 'WSTRING', ARRAY['CodesysIec', 'TwinCat3'], 164, 80, 'string')
ON CONFLICT DO NOTHING;
