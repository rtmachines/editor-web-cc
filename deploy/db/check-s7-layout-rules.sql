-- Check the stored rows against the S7 layout rules (ADR-0147, ADR-0148).
--
-- The query lists the stored rows that the current rule set changes or rejects:
--   nested_array    an array row on a machine with S7_1200 or S7_1500 whose element row is an array
--                   row, and each parameter of a version whose type tree holds such a row
--   layout_changed  a parameter with an S7 address whose type tree holds a struct with a direct SINT
--                   or USINT member, or an array of BOOL with more than one dimension
--
-- The columns are machine_code, reference_number, version_number, tag, part, and type_text.
-- For an array row, reference_number, version_number, and tag are empty and type_text is the
-- array type. For a parameter row, type_text is the type of the parameter.
-- The query reads every machine, also an archived one, every recipe, and every version.
-- It skips the parameters of a discarded draft.
-- A recursive query walks the type tree over the element type and the members of a struct.
-- The owner reads each row. The query writes nothing.
--
-- Run it from the directory deploy/ (docs/ops/runbook.md):
--   docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 < db/check-s7-layout-rules.sql

BEGIN READ ONLY;

WITH RECURSIVE
-- Each type that a parameter uses, with every type below it: the element type of an array and the
-- member types of a struct. UNION drops a type that two paths reach.
tree (root_id, type_id) AS (
    SELECT used.plc_type_id, used.plc_type_id
    FROM (SELECT DISTINCT p.plc_type_id FROM parameter p) used
    UNION
    SELECT tree.root_id, below.type_id
    FROM tree
    CROSS JOIN LATERAL (
        SELECT t.element_type_id AS type_id
        FROM plc_type t
        WHERE t.id = tree.type_id AND t.element_type_id IS NOT NULL
        UNION ALL
        SELECT mem.member_type_id
        FROM plc_type_member mem
        WHERE mem.struct_type_id = tree.type_id
    ) below
),
-- An array row whose element row is an array row (ADR-0148).
nested_row (id) AS (
    SELECT t.id
    FROM plc_type t
    JOIN plc_type e ON e.id = t.element_type_id
    WHERE t.kind = 'array' AND e.kind = 'array'
),
-- A struct row with a direct SINT or USINT member, and an array row of BOOL with more than one
-- dimension (ADR-0147).
changed_row (id) AS (
    SELECT t.id
    FROM plc_type t
    WHERE t.kind = 'struct'
      AND EXISTS (
          SELECT 1
          FROM plc_type_member mem
          JOIN plc_type mt ON mt.id = mem.member_type_id
          WHERE mem.struct_type_id = t.id
            AND mt.kind = 'primitive'
            AND mt.name IN ('SINT', 'USINT')
      )
    UNION ALL
    SELECT t.id
    FROM plc_type t
    JOIN plc_type e ON e.id = t.element_type_id
    WHERE t.kind = 'array'
      AND cardinality(t.dim_lower) > 1
      AND e.kind = 'primitive'
      AND e.name = 'BOOL'
),
-- The head of an array type, for example 'ARRAY[1..3,1..2] OF '.
array_head (id, element_type_id, head) AS (
    SELECT t.id, t.element_type_id,
           'ARRAY[' || (
               SELECT string_agg(d.lo::text || '..' || d.hi::text, ',' ORDER BY d.n)
               FROM unnest(t.dim_lower, t.dim_upper) WITH ORDINALITY AS d (lo, hi, n)
           ) || '] OF '
    FROM plc_type t
    WHERE t.kind = 'array'
),
-- The heads of an array chain from the outer array down to the first row that is not an array.
chain (root_id, current_id, heads) AS (
    SELECT a.id, a.element_type_id, a.head
    FROM array_head a
    UNION ALL
    SELECT c.root_id, a.element_type_id, c.heads || a.head
    FROM chain c
    JOIN array_head a ON a.id = c.current_id
),
-- The text of each type: a name, a name with a string length, or an array chain.
type_label (type_id, label) AS (
    SELECT t.id, t.name || COALESCE('(' || t.string_length::text || ')', '')
    FROM plc_type t
    WHERE t.kind <> 'array'
    UNION ALL
    SELECT c.root_id, c.heads || t.name || COALESCE('(' || t.string_length::text || ')', '')
    FROM chain c
    JOIN plc_type t ON t.id = c.current_id
    WHERE t.kind <> 'array'
),
-- A parameter of a version that is not a discarded draft, with the data that the output shows.
live_parameter AS (
    SELECT p.plc_type_id, p.plc_platform, p.s7_address, p.tag,
           m.code AS machine_code, r.reference_number, v.number AS version_number
    FROM parameter p
    JOIN recipe_version v ON v.id = p.recipe_version_id
    JOIN recipe r ON r.id = v.recipe_id
    JOIN machine m ON m.id = r.machine_id
    WHERE v.deleted_at IS NULL
)
SELECT hit.machine_code, hit.reference_number, hit.version_number, hit.tag, hit.part,
       hit.type_text
FROM (
    SELECT m.code AS machine_code, NULL::text AS reference_number,
           NULL::integer AS version_number, NULL::text AS tag,
           'nested_array' AS part, l.label AS type_text
    FROM nested_row n
    JOIN plc_type t ON t.id = n.id
    JOIN machine m ON m.id = t.machine_id
    JOIN type_label l ON l.type_id = t.id
    WHERE m.plc_platform IN ('S7_1200', 'S7_1500')
    UNION ALL
    SELECT p.machine_code, p.reference_number, p.version_number, p.tag,
           'nested_array', l.label
    FROM live_parameter p
    JOIN type_label l ON l.type_id = p.plc_type_id
    WHERE p.plc_platform IN ('S7_1200', 'S7_1500')
      AND EXISTS (
          SELECT 1
          FROM tree x
          JOIN nested_row n ON n.id = x.type_id
          WHERE x.root_id = p.plc_type_id
      )
    UNION ALL
    SELECT p.machine_code, p.reference_number, p.version_number, p.tag,
           'layout_changed', l.label
    FROM live_parameter p
    JOIN type_label l ON l.type_id = p.plc_type_id
    WHERE p.s7_address IS NOT NULL
      AND EXISTS (
          SELECT 1
          FROM tree x
          JOIN changed_row c ON c.id = x.type_id
          WHERE x.root_id = p.plc_type_id
      )
) hit
ORDER BY hit.machine_code COLLATE "C", hit.reference_number COLLATE "C" NULLS FIRST,
         hit.version_number NULLS FIRST, hit.tag COLLATE "C" NULLS FIRST,
         hit.part COLLATE "C", hit.type_text COLLATE "C";

COMMIT;
