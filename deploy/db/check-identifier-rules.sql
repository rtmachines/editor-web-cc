-- Check the stored type rows against the identifier rules (ADR-0143, ADR-0145).
--
-- The query lists the stored type rows that the current rule set rejects:
--   struct_name_case      a struct row whose machine, name without case, and revision belong to a
--                         row with another spelling. Each spelling gives one row.
--   member_name_case      a member row whose struct and member name without case belong to
--                         another member row. Each spelling gives one row.
--   array_subscript       an array row with a bound array that starts at another subscript than 1
--   reserved_struct_name  a struct row whose name is a reserved word, in any case
--
-- The columns are machine_code, type_name, revision, part, and stored_text. For a member row,
-- type_name and revision are those of the struct and stored_text is the member name. For an array
-- row, type_name names the element type and stored_text holds both bound arrays.
-- The query reads every machine, also an archived one, and every revision.
-- The first three parts stop migration 0011. The part reserved_struct_name is for information:
-- a struct with a reserved name keeps its rows and gets no new revision.
-- The owner reads each row. The query writes nothing.
--
-- Run it from the directory deploy/ (docs/ops/runbook.md):
--   docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 < db/check-identifier-rules.sql

BEGIN READ ONLY;

WITH
-- The reserved words of ADR-0143 (c): the IEC and TwinCAT keywords and the primitive names.
-- One word on each line. A test compares the lines with RESERVED_KEYWORDS and Primitive::ALL.
reserved (word) AS (
    SELECT unnest(ARRAY[
        -- reserved words: begin
        'ABS',
        'ABSTRACT',
        'ACOS',
        'ACTION',
        'ADD',
        'ADR',
        'AND',
        'ANDN',
        'ANY',
        'ANY_BIT',
        'ANY_CHAR',
        'ANY_CHARS',
        'ANY_DATE',
        'ANY_DERIVED',
        'ANY_DURATION',
        'ANY_ELEMENTARY',
        'ANY_INT',
        'ANY_MAGNITUDE',
        'ANY_NUM',
        'ANY_REAL',
        'ANY_SIGNED',
        'ANY_STRING',
        'ANY_UNSIGNED',
        'ARRAY',
        'ASIN',
        'AT',
        'ATAN',
        'BITADR',
        'BOOL',
        'BY',
        'BYTE',
        'CAL',
        'CALC',
        'CALCN',
        'CASE',
        'CHAR',
        'CLASS',
        'CONFIGURATION',
        'CONSTANT',
        'CONTINUE',
        'COS',
        'DATE',
        'DATE_AND_TIME',
        'DINT',
        'DIV',
        'DO',
        'DT',
        'DTL',
        'DWORD',
        'ELSE',
        'ELSIF',
        'EN',
        'END_ACTION',
        'END_CASE',
        'END_CLASS',
        'END_CONFIGURATION',
        'END_FOR',
        'END_FUNCTION',
        'END_FUNCTION_BLOCK',
        'END_GET',
        'END_IF',
        'END_INTERFACE',
        'END_METHOD',
        'END_NAMESPACE',
        'END_PROGRAM',
        'END_PROPERTY',
        'END_REPEAT',
        'END_RESOURCE',
        'END_SET',
        'END_STEP',
        'END_STRUCT',
        'END_TRANSITION',
        'END_TYPE',
        'END_VAR',
        'END_WHILE',
        'ENO',
        'EQ',
        'EXIT',
        'EXP',
        'EXPT',
        'EXTENDS',
        'FALSE',
        'FINAL',
        'FOR',
        'FROM',
        'FUNCTION',
        'FUNCTION_BLOCK',
        'F_EDGE',
        'GE',
        'GET',
        'GT',
        'IF',
        'IMPLEMENTS',
        'INDEXOF',
        'INITIAL_STEP',
        'INT',
        'INTERFACE',
        'INTERNAL',
        'JMP',
        'JMPC',
        'JMPCN',
        'LD',
        'LDATE',
        'LDATE_AND_TIME',
        'LDN',
        'LDT',
        'LE',
        'LINT',
        'LN',
        'LOG',
        'LREAL',
        'LT',
        'LTIME',
        'LTIME_OF_DAY',
        'LTOD',
        'LWORD',
        'MAX',
        'METHOD',
        'MIN',
        'MOD',
        'MOVE',
        'MUL',
        'MUX',
        'NAMESPACE',
        'NE',
        'NON_RETAIN',
        'NOT',
        'NULL',
        'OF',
        'ON',
        'OR',
        'ORN',
        'OVERLAP',
        'OVERRIDE',
        'PARAMS',
        'PERSISTENT',
        'POINTER',
        'PRIVATE',
        'PROGRAM',
        'PROPERTY',
        'PROTECTED',
        'PUBLIC',
        'R',
        'READ_ONLY',
        'READ_WRITE',
        'REAL',
        'REF',
        'REFERENCE',
        'REF_TO',
        'REPEAT',
        'RESOURCE',
        'RET',
        'RETAIN',
        'RETC',
        'RETCN',
        'RETURN',
        'ROL',
        'ROR',
        'R_EDGE',
        'S',
        'S5TIME',
        'SEL',
        'SET',
        'SHL',
        'SHR',
        'SIN',
        'SINT',
        'SIZEOF',
        'SQRT',
        'ST',
        'STEP',
        'STN',
        'STRING',
        'STRUCT',
        'SUB',
        'SUPER',
        'TAN',
        'TASK',
        'THEN',
        'THIS',
        'TIME',
        'TIME_OF_DAY',
        'TO',
        'TOD',
        'TRANSITION',
        'TRUE',
        'TRUNC',
        'TYPE',
        'UDINT',
        'UINT',
        'ULINT',
        'UNTIL',
        'USING',
        'USINT',
        'VAR',
        'VAR_ACCESS',
        'VAR_CONFIG',
        'VAR_EXTERNAL',
        'VAR_GLOBAL',
        'VAR_INPUT',
        'VAR_IN_OUT',
        'VAR_OUTPUT',
        'VAR_STAT',
        'VAR_TEMP',
        'WCHAR',
        'WHILE',
        'WITH',
        'WORD',
        'WSTRING',
        'XOR',
        'XORN'
        -- reserved words: end
    ])
),
-- A struct row, with the machine that owns it.
struct_row AS (
    SELECT t.id, m.code AS machine_code, t.machine_id, t.name, t.revision
    FROM plc_type t
    JOIN machine m ON m.id = t.machine_id
    WHERE t.kind = 'struct'
)
SELECT hit.machine_code, hit.type_name, hit.revision, hit.part, hit.stored_text
FROM (
    SELECT s.machine_code, s.name AS type_name, s.revision,
           'struct_name_case' AS part, s.name AS stored_text
    FROM struct_row s
    WHERE EXISTS (
        SELECT 1
        FROM struct_row o
        WHERE o.machine_id = s.machine_id
          AND o.revision = s.revision
          AND lower(o.name) = lower(s.name)
          AND o.name <> s.name
    )
    UNION ALL
    SELECT s.machine_code, s.name, s.revision, 'member_name_case', mem.member_name
    FROM plc_type_member mem
    JOIN struct_row s ON s.id = mem.struct_type_id
    WHERE EXISTS (
        SELECT 1
        FROM plc_type_member o
        WHERE o.struct_type_id = mem.struct_type_id
          AND lower(o.member_name) = lower(mem.member_name)
          AND o.member_name <> mem.member_name
    )
    UNION ALL
    SELECT m.code, 'ARRAY OF ' || COALESCE(e.name, e.kind), t.revision, 'array_subscript',
           t.dim_lower::text || ' ' || t.dim_upper::text
    FROM plc_type t
    JOIN plc_type e ON e.id = t.element_type_id
    LEFT JOIN machine m ON m.id = t.machine_id
    WHERE t.kind = 'array'
      AND (array_lower(t.dim_lower, 1) IS DISTINCT FROM 1
           OR array_lower(t.dim_upper, 1) IS DISTINCT FROM 1)
    UNION ALL
    SELECT s.machine_code, s.name, s.revision, 'reserved_struct_name', s.name
    FROM struct_row s
    WHERE upper(s.name) IN (SELECT r.word FROM reserved r)
) hit
ORDER BY hit.machine_code COLLATE "C", hit.type_name COLLATE "C", hit.revision,
         hit.part COLLATE "C", hit.stored_text COLLATE "C";

COMMIT;
