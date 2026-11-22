-- Check the stored data against the platform ranges (ADR-0139 risks, ADR-0140 risks, ADR-0180 (l)).
--
-- The query lists the stored rows that the current rule set rejects.
-- The line_break part lists rows that every rule set accepts but the next form save changes.
--
--   value       a parameter on CodesysIec or TwinCat3 whose type tree holds LDATE or LDT,
--               with a value leaf above 9223372036854775807
--   limit_min   a limit of such a parameter with min_value above 9223372036854775807
--   limit_max   a limit of such a parameter with max_value above 9223372036854775807
--   s7_address  a parameter on S7_1200 or S7_1500 with a data block number above 59999
--   wide_text   a parameter on S7_1200 or S7_1500 whose type tree holds WCHAR or WSTRING, with a
--               text leaf that the Siemens page refuses (ADR-0180): a character above U+D7FF in a
--               WCHAR leaf, a character at U+D000 or above in a WSTRING leaf.
--               An Approved version of such a parameter becomes not loadable after the upgrade.
--   iec_wide_text  a WSTRING leaf on CodesysIec or TwinCat3 with a character above U+FFFF
--               (ADR-0201 (k)).
--               ADR-0201 (a) limits a WSTRING on these platforms to UCS-2. An Approved version of
--               such a parameter becomes not loadable after the upgrade.
--   line_break    a text leaf with LF or CR on any platform (ADR-0210).
--
-- The query reads every version status and also soft-deleted rows: a deleted draft, a deleted
-- recipe, and an archived machine. It finds a value leaf at each depth: a struct member, a member
-- of a nested struct, and an array element. The date range check keeps only the number leaves.
-- A ULINT, LWORD, LTIME, REAL, or LREAL leaf in the same type tree can give a false hit in its
-- value row or in a limit row of that leaf.
-- A wide_text row shows the text leaf. The query matches each text leaf with its declared type
-- and uses the limit of that type. A struct with a WCHAR member and a WSTRING member gives no
-- false hit.
-- The owner reads each row. The query writes nothing.
--
-- Run it from the directory deploy/ (docs/ops/runbook.md):
--   docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 \
--     < db/check-platform-ranges.sql

BEGIN READ ONLY;

WITH RECURSIVE
-- One edge for each array element type and each struct member.
edge (parent_id, child_id) AS (
    SELECT t.id, t.element_type_id
    FROM plc_type t
    WHERE t.element_type_id IS NOT NULL
    UNION ALL
    SELECT m.struct_type_id, m.member_type_id
    FROM plc_type_member m
),
-- Every type that the type tree of a parameter type reaches, with the type itself.
reach (root_id, type_id) AS (
    SELECT p.plc_type_id, p.plc_type_id
    FROM parameter p
    GROUP BY p.plc_type_id
    UNION
    SELECT r.root_id, e.child_id
    FROM reach r
    JOIN edge e ON e.parent_id = r.type_id
),
-- The types whose type tree holds LDATE or LDT.
date_root (root_id) AS (
    SELECT r.root_id
    FROM reach r
    JOIN plc_type t ON t.id = r.type_id
    WHERE t.kind = 'primitive' AND t.name IN ('LDATE', 'LDT')
    GROUP BY r.root_id
),
-- The types whose type tree holds CHAR, WCHAR, STRING, or WSTRING.
text_root (root_id) AS (
    SELECT r.root_id
    FROM reach r
    JOIN plc_type t ON t.id = r.type_id
    WHERE t.kind = 'primitive' AND t.name IN ('CHAR', 'WCHAR', 'STRING', 'WSTRING')
    GROUP BY r.root_id
),
-- Each value node of a parameter with a text type in its type tree carries its declared type.
-- A struct node is a JSON object with one key for each member.
-- An array node has one level of JSON arrays for each dimension.
-- The column levels counts the array levels that remain, and is NULL outside an array.
walk (parameter_id, node, type_id, levels) AS (
    SELECT p.id, p.value, p.plc_type_id, NULL::integer
    FROM parameter p
    JOIN text_root t ON t.root_id = p.plc_type_id
    UNION ALL
    SELECT w.parameter_id, c.node, c.type_id, c.levels
    FROM walk w
    JOIN plc_type t ON t.id = w.type_id
    CROSS JOIN LATERAL (
        SELECT w.node -> m.member_name, m.member_type_id, NULL::integer
        FROM plc_type_member m
        WHERE t.kind = 'struct'
          AND jsonb_typeof(w.node) = 'object'
          AND m.struct_type_id = t.id
        UNION ALL
        SELECT e.item,
               CASE WHEN a.remaining > 1 THEN t.id ELSE t.element_type_id END,
               CASE WHEN a.remaining > 1 THEN a.remaining - 1 END
        FROM (SELECT coalesce(w.levels, cardinality(t.dim_lower)) AS remaining) a
        CROSS JOIN LATERAL jsonb_array_elements(
            CASE WHEN t.kind = 'array' AND jsonb_typeof(w.node) = 'array' THEN w.node ELSE '[]'::jsonb END
        ) AS e (item)
    ) AS c (node, type_id, levels)
),
-- The text leaves with their declared types.
text_leaf (parameter_id, type_name, leaf_text) AS (
    SELECT w.parameter_id,
           t.name,
           w.node #>> '{}'
    FROM walk w
    JOIN plc_type t ON t.id = w.type_id
    WHERE t.kind = 'primitive'
      AND t.name IN ('CHAR', 'WCHAR', 'STRING', 'WSTRING')
      AND jsonb_typeof(w.node) = 'string'
),
located AS (
    SELECT p.id AS parameter_id,
           p.plc_platform,
           p.plc_type_id,
           p.tag,
           p.s7_address,
           p.value,
           m.code AS machine_code,
           r.reference_number,
           v.number AS version_number,
           v.status AS version_status
    FROM parameter p
    JOIN recipe_version v ON v.id = p.recipe_version_id
    JOIN recipe r ON r.id = v.recipe_id
    JOIN machine m ON m.id = r.machine_id
)
SELECT hit.machine_code, hit.reference_number, hit.version_number, hit.version_status,
       hit.tag, hit.part, hit.stored_text
FROM (
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           'value' AS part, leaf.value::text AS stored_text
    FROM located l
    JOIN date_root d ON d.root_id = l.plc_type_id
    CROSS JOIN LATERAL jsonb_path_query(
        l.value,
        'strict $.** ? (@.type() == "number" && @ > 9223372036854775807)'
    ) AS leaf (value)
    WHERE l.plc_platform IN ('CodesysIec', 'TwinCat3')
    UNION ALL
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           'limit_min', pl.min_value::text
    FROM located l
    JOIN date_root d ON d.root_id = l.plc_type_id
    JOIN parameter_limit pl ON pl.parameter_id = l.parameter_id
    WHERE l.plc_platform IN ('CodesysIec', 'TwinCat3')
      AND pl.min_value > 9223372036854775807
    UNION ALL
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           'limit_max', pl.max_value::text
    FROM located l
    JOIN date_root d ON d.root_id = l.plc_type_id
    JOIN parameter_limit pl ON pl.parameter_id = l.parameter_id
    WHERE l.plc_platform IN ('CodesysIec', 'TwinCat3')
      AND pl.max_value > 9223372036854775807
    UNION ALL
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           's7_address', l.s7_address
    FROM located l
    WHERE l.plc_platform IN ('S7_1200', 'S7_1500')
      AND substring(l.s7_address FROM '^DB([0-9]+)\.')::numeric > 59999
    UNION ALL
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           'wide_text', w.leaf_text
    FROM located l
    JOIN text_leaf w ON w.parameter_id = l.parameter_id
    WHERE l.plc_platform IN ('S7_1200', 'S7_1500')
      AND w.type_name IN ('WCHAR', 'WSTRING')
      AND EXISTS (
          SELECT 1
          FROM regexp_split_to_table(w.leaf_text, '') AS c (character)
          -- ADR-0180 refuses U+D000 and above for WSTRING and U+D800 and above for WCHAR.
          WHERE ascii(c.character) >= CASE WHEN w.type_name = 'WSTRING' THEN 53248 ELSE 55296 END
      )
    UNION ALL
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           'iec_wide_text', w.leaf_text
    FROM located l
    JOIN text_leaf w ON w.parameter_id = l.parameter_id
    WHERE l.plc_platform IN ('CodesysIec', 'TwinCat3')
      AND w.type_name = 'WSTRING'
      AND EXISTS (
          SELECT 1
          FROM regexp_split_to_table(w.leaf_text, '') AS c (character)
          WHERE ascii(c.character) > 65535
      )
    UNION ALL
    SELECT l.machine_code, l.reference_number, l.version_number, l.version_status, l.tag,
           'line_break', t.leaf_text
    FROM located l
    JOIN text_leaf t ON t.parameter_id = l.parameter_id
    WHERE strpos(t.leaf_text, chr(10)) > 0 OR strpos(t.leaf_text, chr(13)) > 0
) hit
ORDER BY hit.machine_code, hit.reference_number, hit.version_number, hit.tag, hit.part;

COMMIT;
