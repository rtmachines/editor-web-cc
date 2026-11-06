"""Check ADR-0136 forms with shared migration rewrite test helpers."""

import importlib.util
import pathlib
import unittest


_spec = importlib.util.spec_from_file_location(
    "test_migration_rewrite", pathlib.Path(__file__).with_name("test_migration_rewrite.py")
)
_tests = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_tests)

BASE = _tests.BASE
CHECK = _tests.CHECK
TYPE = _tests.TYPE
DEFAULT = _tests.DEFAULT
NOT_NULL = _tests.NOT_NULL
MIGRATIONS = _tests.MIGRATIONS
STORED = "stored generated column"
CONSTRAINT = "constraint without not valid"
INDEX = "index without concurrently"
UNIQUE = "unique without using index"
PRIMARY_KEY = "primary key"
EXCLUSION = "exclusion constraint"
VACUUM_FULL = "vacuum full"
CLUSTER = "cluster"
TABLESPACE = "set tablespace"
LOGGED = "set logged"
ACCESS_METHOD = "set access method"
DOMAIN_CONSTRAINT = "domain constraint without not valid"
DOMAIN_VALIDATE = "domain validate"
LOCK = "lock table"
CONDITION_BODIES = (
    ("BEGIN IF (SELECT CASE WHEN a THEN true END) THEN ", "END IF; END"),
    ("BEGIN IF CASE WHEN a THEN true ELSE false END THEN ", "END IF; END"),
    ("BEGIN IF (SELECT CASE WHEN a THEN CASE WHEN b THEN true END END) THEN ", "END IF; END"),
    ("BEGIN IF false THEN NULL; ELSIF (SELECT CASE WHEN a THEN true END) THEN ", "END IF; END"),
    ("BEGIN IF false THEN NULL; ELSIF CASE WHEN a THEN true END THEN ", "END IF; END"),
    ("BEGIN WHILE (SELECT CASE WHEN a THEN true END) LOOP ", "END LOOP; END"),
    ("BEGIN WHILE CASE WHEN a THEN true END LOOP ", "END LOOP; END"),
    ("BEGIN WHILE flags[CASE WHEN a THEN 1 ELSE 2 END] LOOP ", "END LOOP; END"),
)
DECLARE_BODIES = (
    ("DECLARE BEGIN ", "END"),
    ("<<l>> DECLARE BEGIN ", "END l"),
    ("DECLARE n int; BEGIN ", "END"),
    ("<<l>> DECLARE n int; BEGIN ", "END l"),
)


def new_form_cases():
    """Return SQL fixtures with the diagnostic names of design D1."""
    cases = [
        (STORED, f"ALTER TABLE recipe ADD {column}s integer GENERATED ALWAYS AS (length(note)) STORED")
        for column in ("COLUMN ", "")
    ]
    for name in ("", "CONSTRAINT c "):
        for clause, form in (("CHECK (note IS NOT NULL)", CONSTRAINT),
                             ("FOREIGN KEY (note) REFERENCES recipe (note)", CONSTRAINT),
                             ("UNIQUE (note)", UNIQUE), ("PRIMARY KEY (note)", PRIMARY_KEY),
                             ("UNIQUE (note) USING INDEX TABLESPACE pg_default", UNIQUE),
                             ("PRIMARY KEY (note) USING INDEX TABLESPACE pg_default", PRIMARY_KEY),
                             ("PRIMARY KEY USING INDEX recipe_idx", PRIMARY_KEY),
                             ("EXCLUDE USING btree (note WITH =)", EXCLUSION)):
            cases.append((form, f"ALTER TABLE recipe ADD {name}{clause}"))
    for clause, form in (("CHECK (s > 0)", CONSTRAINT), ("REFERENCES recipe (note)", CONSTRAINT),
                         ("UNIQUE", UNIQUE), ("PRIMARY KEY", PRIMARY_KEY)):
        cases.append((form, f"ALTER TABLE recipe ADD COLUMN s integer {clause}"))
    cases.extend((INDEX, sql) for sql in (
        "CREATE INDEX recipe_idx ON recipe (note)", "CREATE UNIQUE INDEX recipe_idx ON recipe (note)",
        "CREATE INDEX IF NOT EXISTS recipe_idx ON recipe (note)", "CREATE INDEX ON recipe (note)",
        "CREATE INDEX recipe_idx ON ONLY recipe (note)",
    ))
    cases.extend((VACUUM_FULL, f"VACUUM {option} recipe")
                 for option in ("FULL", "(FULL)", "(VERBOSE, FULL)"))
    cases.extend((form, sql) for form, sql in (
        (CLUSTER, "CLUSTER recipe USING recipe_idx"),
        (CLUSTER, "CLUSTER recipe_idx ON recipe"), (CLUSTER, "CLUSTER recipe"),
        (TABLESPACE, "ALTER TABLE recipe SET TABLESPACE pg_default"),
        (LOGGED, "ALTER TABLE recipe SET LOGGED"), (LOGGED, "ALTER TABLE recipe SET UNLOGGED"),
        (ACCESS_METHOD, "ALTER TABLE recipe SET ACCESS METHOD heap"),
        (LOCK, "LOCK recipe"), (LOCK, "LOCK TABLE ONLY recipe IN SHARE MODE"),
        (CONSTRAINT, "ALTER TABLE recipe ADD CHECK (NOT valid)"),
        (DOMAIN_CONSTRAINT, "ALTER DOMAIN label ADD CHECK (VALUE <> '')"),
        (DOMAIN_CONSTRAINT, "ALTER DOMAIN label ADD CONSTRAINT c CHECK (VALUE <> '')"),
        (DOMAIN_CONSTRAINT, "ALTER DOMAIN label SET NOT NULL"),
        (DOMAIN_VALIDATE, "ALTER DOMAIN label VALIDATE CONSTRAINT c"),
    ))
    return cases


class MigrationRewriteFormsTests(_tests.MigrationRewriteTestCase):
    def test_stored_generated_column_fails(self):
        self.assert_change(
            "ALTER TABLE recipe ADD COLUMN s integer GENERATED ALWAYS AS (length(note)) STORED;\n", STORED,
        )

    def test_serial_in_column_or_constraint_name_is_not_a_type(self):
        self.assert_change(
            "ALTER TABLE recipe ADD COLUMN serial integer,\n"
            " ADD COLUMN s integer CONSTRAINT bigserial CHECK (s > 0);\n", CONSTRAINT,
        )

    def test_check_without_not_valid_fails(self):
        self.assert_change(CHECK.removesuffix(" NOT VALID") + ";\n", CONSTRAINT)

    def test_exception_handler_type_change_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
            "BEGIN NULL; EXCEPTION WHEN others THEN "
            "ALTER TABLE recipe ALTER COLUMN note TYPE int USING 1; END$$;\n", TYPE,
        )

    def test_searched_case_index_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
            "BEGIN CASE WHEN true THEN CREATE INDEX x ON recipe (note); "
            "END CASE; END$$;\n", INDEX,
        )

    def test_simple_case_executed_lock_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
            "BEGIN CASE 1 WHEN 1 THEN EXECUTE 'LOCK TABLE recipe'; "
            "END CASE; END$$;\n", LOCK,
        )

    def test_labeled_block_type_change_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
            "<<blk>> BEGIN ALTER TABLE recipe ALTER COLUMN note TYPE integer USING 1; "
            "END blk$$;\n", TYPE,
        )

    def test_exception_handler_index_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
            "BEGIN NULL; EXCEPTION WHEN others THEN "
            "CREATE INDEX x ON recipe (note); END$$;\n", INDEX,
        )

    def assert_body_boundary_forms(self, bodies):
        for prefix, suffix in bodies:
            for statement, form in (
                ("ALTER TABLE recipe ALTER COLUMN note TYPE int USING 1", TYPE),
                ("CREATE INDEX i ON recipe (note)", INDEX),
                ("LOCK TABLE recipe", LOCK),
            ):
                with self.subTest(prefix=prefix, form=form):
                    self.assert_change(
                        "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
                        + prefix + statement + "; " + suffix + "$$;\n", form,
                    )

    def test_nested_case_conditions_preserve_body_forms(self):
        self.assert_body_boundary_forms(CONDITION_BODIES)

    def test_declare_sections_preserve_body_forms(self):
        self.assert_body_boundary_forms(DECLARE_BODIES)

    def test_condition_expressions_do_not_start_body_forms(self):
        for prefix, suffix in CONDITION_BODIES + DECLARE_BODIES:
            with self.subTest(prefix=prefix):
                self.assert_change(
                    "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
                    + prefix + "SELECT cluster, vacuum, lock FROM recipe; " + suffix + "$$;\n"
                )
        for expression in (
            "(CASE WHEN true THEN cluster ELSE vacuum END)",
            "values[CASE WHEN true THEN cluster ELSE vacuum END]",
        ):
            with self.subTest(expression=expression):
                self.assert_change(
                    "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
                    f"BEGIN SELECT {expression} FROM recipe; END$$;\n"
                )

    def test_old_alter_forms_after_body_prefixes_fail(self):
        for prefix, suffix in (
            ("BEGIN NULL; EXCEPTION WHEN others THEN ", "END"),
            ("BEGIN CASE WHEN true THEN ", "END CASE; END"),
            ("<<blk>> BEGIN ", "END blk"),
        ) + CONDITION_BODIES + DECLARE_BODIES:
            for statement, form in (
                ("ALTER TABLE recipe ALTER COLUMN note TYPE TEXT", TYPE),
                ("ALTER TABLE recipe ADD COLUMN token UUID DEFAULT gen_random_uuid()", DEFAULT),
                ("ALTER TABLE recipe ALTER COLUMN note SET NOT NULL", NOT_NULL),
            ):
                with self.subTest(prefix=prefix, form=form):
                    self.assert_change(
                        "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$"
                        + prefix + statement + "; " + suffix + "$$;\n", form,
                    )

    def test_new_forms_follow_object_and_body_rules(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "SELECT cluster, vacuum, lock FROM recipe; END$$;\n"
        )
        base = BASE + "CREATE DOMAIN label AS TEXT;\n"
        for form, statement in new_form_cases():
            sql = statement + ";\n"
            contexts = [(sql, 1), (base + sql, None),
                ("CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
                 + sql + "END$$;\n", 1),
                ("CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
                 + "EXECUTE '" + statement.replace("'", "''") + "';\nEND$$;\n", 1)]
            if form not in (DOMAIN_CONSTRAINT, DOMAIN_VALIDATE):
                contexts.append((BASE.replace("CREATE TABLE", "CREATE TABLE IF NOT EXISTS") + sql, 2))
            for change, line in contexts:
                with self.subTest(form=form, sql=change):
                    self.assert_files({"0001_base.sql": base, "0002_change.sql": change},
                        [("0002_change.sql", line, form)] if line else [])

    def test_domain_validate_body_creation_does_not_exempt(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "CREATE DOMAIN label AS text; ALTER DOMAIN label VALIDATE CONSTRAINT c; END$$;\n",
            DOMAIN_VALIDATE,
        )

    def test_domain_move_keeps_same_file_exemption(self):
        for action, target in (("RENAME TO e", "e"), ("SET SCHEMA staging", "staging.d")):
            with self.subTest(action=action):
                self.assert_change(
                    "CREATE SCHEMA staging;\nCREATE DOMAIN d AS text;\n"
                    f"ALTER DOMAIN d {action};\nALTER DOMAIN {target} SET NOT NULL;\n"
                )

    def test_domain_move_does_not_exempt_reused_name(self):
        for move, reuse in (("RENAME TO e", "label RENAME TO d"),
                            ("SET SCHEMA staging", "archived.d SET SCHEMA public")):
            with self.subTest(move=move):
                self.assert_files({"0001_base.sql":
                    "CREATE SCHEMA staging; CREATE SCHEMA archived;\n"
                    "CREATE DOMAIN archived.d AS text;\n"
                    "CREATE DOMAIN label AS text;\n", "0002_change.sql":
                    f"CREATE DOMAIN d AS text;\nALTER DOMAIN d {move};\n"
                    f"ALTER DOMAIN {reuse};\nALTER DOMAIN d SET NOT NULL;\n"
                }, [("0002_change.sql", 4, DOMAIN_CONSTRAINT)])

    def test_safe_forms_pass_and_check_expression_does_not_prove_not_valid(self):
        for sql in (
            CHECK, "ALTER TABLE recipe ADD CHECK (note IS NOT NULL) NOT VALID",
            "ALTER TABLE recipe ADD FOREIGN KEY (note) REFERENCES recipe (note) NOT VALID",
            "ALTER TABLE recipe ADD CONSTRAINT fk FOREIGN KEY (note) REFERENCES recipe (note) NOT VALID",
            "ALTER TABLE recipe ADD CONSTRAINT u UNIQUE USING INDEX recipe_idx",
            "ALTER TABLE recipe ADD UNIQUE USING INDEX recipe_x",
            'ALTER TABLE recipe ADD CONSTRAINT u UNIQUE USING INDEX "tablespace"',
            "CREATE INDEX CONCURRENTLY recipe_idx ON recipe (note)", "VACUUM recipe",
            "VACUUM (ANALYZE) recipe", "VACUUM (FULL false) recipe",
            "ALTER DOMAIN label ADD CONSTRAINT c CHECK (VALUE <> '') NOT VALID",
        ):
            with self.subTest(sql=sql):
                self.assert_change(sql + ";\n")
        self.assert_change("ALTER TABLE recipe ADD CHECK (NOT valid);\n", CONSTRAINT)

    def test_vacuum_full_false_values_pass(self):
        for value in ("false", "off", "0", "no"):
            for spelling in (value, value.upper()):
                for quote in ("", "'"):
                    with self.subTest(value=spelling, quote=quote):
                        self.assert_change(f"VACUUM (FULL {quote}{spelling}{quote}) recipe;\n")

    def test_vacuum_full_true_values_report_form(self):
        for value in ("true", "on", "1", "yes"):
            for spelling in (value, value.upper()):
                for quote in ("", "'"):
                    with self.subTest(value=spelling, quote=quote):
                        self.assert_change(f"VACUUM (FULL {quote}{spelling}{quote}) recipe;\n", VACUUM_FULL)

    def test_legacy_cluster_on_new_table_passes(self):
        self.assert_files({"0001_new.sql":
            "CREATE TABLE t (n int); CREATE INDEX i ON t (n); CLUSTER i ON t;\n"
        })

    def test_global_maintenance_is_not_exempt_after_table_creation(self):
        for sql, form in (("VACUUM FULL", VACUUM_FULL), ("CLUSTER", CLUSTER)):
            with self.subTest(sql=sql):
                self.assert_change(sql + ";\n", form)
            with self.subTest(sql=sql, created=True):
                (self.directory / "0002_change.sql").unlink()
                self.assert_files({"0001_base.sql": BASE + sql + ";\n"},
                                  [("0001_base.sql", 2, form)])

    def test_repeated_constraint_actions_report_each_form_once(self):
        self.assert_files({"0001_base.sql": BASE, "0002_change.sql":
            "ALTER TABLE recipe ADD CHECK (length(note) > 0), ADD UNIQUE (note),\n"
            " ADD CONSTRAINT other_check CHECK (length(other) > 0);\n"
        }, [("0002_change.sql", 1, form) for form in (CONSTRAINT, UNIQUE)])

    def test_shipped_index_without_exemption_fails(self):
        self.write_files({path.name: path.read_text(encoding="utf-8")
                          for path in MIGRATIONS.glob("*.sql")
                          if path.name[:4] in {f"{number:04d}" for number in range(1, 11)}})
        result = self.run_with_exempt({})
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.diagnostics(result.stdout), [("0006_operations.sql", 10, INDEX)])


if __name__ == "__main__":
    unittest.main()
