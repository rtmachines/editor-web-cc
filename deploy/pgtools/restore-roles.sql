-- restore-roles.sql: one line for each value that decides who may reach a cluster.
--
-- The file restore-roles.expected holds the lines of the release (ADR-0103 (m)).
-- The script restore-volume runs this file as the superuser postgres in the database recipe of
-- its new cluster. The storage test of recipe-db runs it on the harness database after
-- 01-roles.sh and every migration.
--
-- A line holds KIND|FIELD|...|FIELD, with a bar between the fields, as tests/grants.expected does.
-- No line holds an OID or a password hash. Two clusters have other OIDs, and a password hash must
-- never reach the event log. A role, a database, or a grantee always appears by its name.
--
-- Kinds, in the order of the output:
--   ACL      ACL|database|grantor|grantee|privilege|grantable. One line for each item of the access
--            list of the databases postgres, template1, and recipe. The grantee PUBLIC is a name.
--            A database with no list gets the default list of its owner.
--   GRANT    GRANT|name|privilege. The grants of recipe_app in the format of tests/grants.expected.
--   HBA      HBA|rule. One line for each rule of pg_hba.conf. The line has no comment and no blank.
--            The fields of a rule are joined by one space, so the padding of the subnet does not
--            count. The lines keep the order of the file, because the first matching rule wins.
--   MEMBER   MEMBER|role|member|grantor|admin|inherit|set. One line for each membership.
--   ROLE     ROLE|name|super|inherit|createrole|createdb|login|replication|bypassrls|connlimit|until.
--            The query reads these columns only. The column until is a dash when it is NULL.
--   SETTING  SETTING|database|role|setting. One line for each setting of ALTER ROLE or
--            ALTER DATABASE. A dash means all databases or all roles.
-- The lines of every kind except HBA are sorted in byte order.
--
-- The search path makes the names of the routines in the GRANT lines independent of the session.
-- It must show the schema public without a prefix, as in tests/grants.expected.
SET search_path = pg_catalog, public;

WITH access (kind, position, line) AS (
    SELECT 'ACL', 0,
           concat_ws('|', 'ACL', d.datname, pg_get_userbyid(x.grantor),
                     CASE x.grantee WHEN 0 THEN 'PUBLIC' ELSE pg_get_userbyid(x.grantee) END,
                     x.privilege_type, x.is_grantable)
    FROM pg_database AS d
    CROSS JOIN LATERAL aclexplode(COALESCE(d.datacl, acldefault('d', d.datdba))) AS x
    WHERE d.datname IN ('postgres', 'template1', 'recipe')
    UNION ALL
    SELECT 'HBA', r.position,
           'HBA|' || array_to_string(regexp_split_to_array(r.rule, '\s+'), ' ')
    FROM (SELECT trim(both E' \t\r' FROM l.raw) AS rule, l.position
          FROM regexp_split_to_table(pg_read_file('pg_hba.conf'), E'\n')
               WITH ORDINALITY AS l (raw, position)) AS r
    WHERE r.rule <> '' AND r.rule NOT LIKE '#%'
    UNION ALL
    SELECT 'ROLE', 0,
           concat_ws('|', 'ROLE', a.rolname, a.rolsuper, a.rolinherit, a.rolcreaterole,
                     a.rolcreatedb, a.rolcanlogin, a.rolreplication, a.rolbypassrls,
                     a.rolconnlimit, COALESCE(a.rolvaliduntil::text, '-'))
    FROM pg_authid AS a
    UNION ALL
    SELECT 'MEMBER', 0,
           concat_ws('|', 'MEMBER', pg_get_userbyid(m.roleid), pg_get_userbyid(m.member),
                     pg_get_userbyid(m.grantor), m.admin_option, m.inherit_option, m.set_option)
    FROM pg_auth_members AS m
    UNION ALL
    SELECT 'SETTING', 0,
           concat_ws('|', 'SETTING',
                     COALESCE((SELECT datname FROM pg_database WHERE oid = s.setdatabase), '-'),
                     CASE s.setrole WHEN 0 THEN '-' ELSE pg_get_userbyid(s.setrole) END,
                     c.setting)
    FROM pg_db_role_setting AS s
    CROSS JOIN LATERAL unnest(s.setconfig) AS c (setting)
    UNION ALL
    SELECT 'GRANT', 0, 'GRANT|' || g.line
    FROM (SELECT t.table_name::text || '|' || t.privilege_type::text AS line
          FROM information_schema.role_table_grants AS t
          WHERE t.grantee = 'recipe_app' AND t.table_schema = 'public'
          UNION ALL
          SELECT c.relname::text || '.' || a.attname::text || '|' || x.privilege_type
          FROM pg_attribute AS a
          JOIN pg_class AS c ON c.oid = a.attrelid
          CROSS JOIN LATERAL aclexplode(a.attacl) AS x
          WHERE x.grantee = 'recipe_app'::regrole
            AND c.relnamespace = 'public'::regnamespace AND NOT a.attisdropped
          UNION ALL
          SELECT p.oid::regprocedure::text || '|' || r.privilege_type::text
          FROM information_schema.role_routine_grants AS r
          JOIN pg_proc AS p ON p.pronamespace = 'public'::regnamespace
           AND pg_catalog.nameconcatoid(p.proname, p.oid)::text = r.specific_name::text
          WHERE r.grantee = 'recipe_app' AND r.specific_schema = 'public') AS g
)
SELECT line
FROM access
ORDER BY kind COLLATE "C", position, line COLLATE "C";
