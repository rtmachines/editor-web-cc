-- restore-code.sql: one line for each object of a dump that runs code.
--
-- The line holds KIND|NAME|OWNER|SHA256, with a bar between the fields, as tests/grants.expected does.
-- The file restore-code.expected holds the lines of the release (ADR-0067 (i), ADR-0076 (a)).
-- The script restore-test runs this file on the scratch cluster. The storage test of recipe-db
-- runs it on the harness database after 01-roles.sh and every migration (ADR-0067 (j)).
--
-- Kinds, all outside pg_catalog and information_schema:
--   FUNCTION       a function. The hash covers pg_get_functiondef.
--   CHECK          a CHECK constraint of a table or a domain. The hash covers pg_get_constraintdef.
--   GENERATED      a generated column. The hash covers the expression of pg_get_expr.
--   TRIGGER        a trigger with NOT tgisinternal. The hash covers pg_get_triggerdef and tgenabled,
--                  with a line feed between them (ADR-0078 (a)).
--   EVENT TRIGGER  an event trigger. The hash covers the event, the tags, the function, and the state.
-- NAME is the schema and the tag of the entry of pg_restore -l, with one space between them. The
-- tag of a trigger is the table and the trigger. The schema of an event trigger is a dash
-- (ADR-0076 (b)). The lines are sorted in byte order.
--
-- The empty search path makes the text of the definitions independent of the session. A type of
-- another schema is qualified, as pg_dump writes it.
SET search_path = pg_catalog;

WITH code (kind, name, owner, definition) AS (
    SELECT 'FUNCTION',
           n.nspname || ' ' || p.proname || '('
               || COALESCE((SELECT string_agg(format_type(t.oid, NULL), ', ' ORDER BY a.position)
                            FROM unnest(p.proargtypes::oid[]) WITH ORDINALITY AS a (oid, position)
                            JOIN pg_type AS t ON t.oid = a.oid), '')
               || ')',
           pg_get_userbyid(p.proowner),
           pg_get_functiondef(p.oid)
    FROM pg_proc AS p
    JOIN pg_namespace AS n ON n.oid = p.pronamespace
    WHERE p.prokind IN ('f', 'w')
      AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
      AND NOT EXISTS (SELECT 1 FROM pg_depend AS d
                      WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e')
    UNION ALL
    SELECT 'CHECK', n.nspname || ' ' || c.relname || ' ' || k.conname,
           pg_get_userbyid(c.relowner), pg_get_constraintdef(k.oid)
    FROM pg_constraint AS k
    JOIN pg_class AS c ON c.oid = k.conrelid
    JOIN pg_namespace AS n ON n.oid = c.relnamespace
    WHERE k.contype = 'c'
      AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
    UNION ALL
    SELECT 'CHECK', n.nspname || ' ' || t.typname || ' ' || k.conname,
           pg_get_userbyid(t.typowner), pg_get_constraintdef(k.oid)
    FROM pg_constraint AS k
    JOIN pg_type AS t ON t.oid = k.contypid
    JOIN pg_namespace AS n ON n.oid = t.typnamespace
    WHERE k.contype = 'c'
      AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
    UNION ALL
    SELECT 'GENERATED', n.nspname || ' ' || c.relname || ' ' || a.attname,
           pg_get_userbyid(c.relowner), pg_get_expr(d.adbin, d.adrelid)
    FROM pg_attribute AS a
    JOIN pg_class AS c ON c.oid = a.attrelid
    JOIN pg_namespace AS n ON n.oid = c.relnamespace
    JOIN pg_attrdef AS d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
    WHERE a.attgenerated <> '' AND NOT a.attisdropped
      AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
    UNION ALL
    SELECT 'TRIGGER', n.nspname || ' ' || c.relname || ' ' || g.tgname,
           pg_get_userbyid(c.relowner), concat_ws(E'\n', pg_get_triggerdef(g.oid), g.tgenabled)
    FROM pg_trigger AS g
    JOIN pg_class AS c ON c.oid = g.tgrelid
    JOIN pg_namespace AS n ON n.oid = c.relnamespace
    WHERE NOT g.tgisinternal
      AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
    UNION ALL
    SELECT 'EVENT TRIGGER', '- ' || e.evtname, pg_get_userbyid(e.evtowner),
           concat_ws(E'\n', e.evtevent, COALESCE(array_to_string(e.evttags, ','), ''),
                     e.evtfoid::regprocedure::text, e.evtenabled)
    FROM pg_event_trigger AS e
)
SELECT l.line
FROM (SELECT kind || '|' || name || '|' || owner || '|'
                 || encode(sha256(convert_to(definition, 'UTF8')), 'hex') AS line
      FROM code) AS l
ORDER BY l.line COLLATE "C";
