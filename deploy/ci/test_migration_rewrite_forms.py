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
CONSTRAINED_DOMAIN = "add column of a constrained domain"
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
    def test_multiple_add_clauses_with_constrained_domain_report_one_form(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN a int,\n"
            " ADD COLUMN w pos_int;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_constrained_domain_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_constrained_domain_with_default_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int DEFAULT 1;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_matching_constrained_domain_exemption_passes(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN w pos_int;\n", CONSTRAINED_DOMAIN, line=2,
        )
        result = self.run_with_exempt({("0002_change.sql", CONSTRAINED_DOMAIN): "ADR-0153"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")

    def test_add_column_of_constrained_domain_array_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int[];\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_constrained_domain_without_column_keyword_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD weight pos_int;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_constrained_domain_if_not_exists_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN IF NOT EXISTS weight pos_int;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_constrained_domain_if_not_exists_without_column_keyword_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD IF NOT EXISTS w pos_int;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_schema_qualified_constrained_domain_fails(self):
        self.assert_change(
            "CREATE SCHEMA staging;\n"
            "CREATE DOMAIN staging.pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN weight staging.pos_int;\n", CONSTRAINED_DOMAIN, line=3,
        )

    def test_add_column_of_quoted_constrained_domain_fails(self):
        self.assert_change(
            'CREATE DOMAIN "PosInt" AS INTEGER CHECK (VALUE > 0);\n'
            'ALTER TABLE recipe ADD COLUMN weight "PosInt";\n', CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_constrained_domain_in_function_body_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int; END$$;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def assert_conditional_body_domain_change(self, definition, change, form):
        self.assert_change(
            f"CREATE DOMAIN probe_type AS {definition};\n"
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            f"IF false THEN {change}; END IF;\n"
            "ALTER TABLE recipe ADD COLUMN probe_value probe_type; END$$;\n", form, line=2,
        )

    def test_body_conditional_drop_keeps_constrained_domain(self):
        self.assert_conditional_body_domain_change(
            "integer CHECK (VALUE > 0)", "DROP DOMAIN probe_type", CONSTRAINED_DOMAIN,
        )

    def test_body_conditional_rename_keeps_constrained_domain(self):
        self.assert_conditional_body_domain_change(
            "integer CHECK (VALUE > 0)", "ALTER DOMAIN probe_type RENAME TO renamed",
            CONSTRAINED_DOMAIN,
        )

    def test_body_conditional_set_schema_keeps_constrained_domain(self):
        self.assert_conditional_body_domain_change(
            "integer CHECK (VALUE > 0)", "ALTER TYPE probe_type SET SCHEMA staging",
            CONSTRAINED_DOMAIN,
        )

    def test_body_conditional_drop_keeps_volatile_domain(self):
        self.assert_conditional_body_domain_change(
            "double precision DEFAULT random()", "DROP DOMAIN probe_type", DEFAULT,
        )

    def test_body_conditional_rename_keeps_volatile_domain(self):
        self.assert_conditional_body_domain_change(
            "double precision DEFAULT random()", "ALTER DOMAIN probe_type RENAME TO renamed",
            DEFAULT,
        )

    def test_body_conditional_set_schema_keeps_volatile_domain(self):
        self.assert_conditional_body_domain_change(
            "double precision DEFAULT random()", "ALTER TYPE probe_type SET SCHEMA staging",
            DEFAULT,
        )

    def test_body_renamed_constrained_domain_counts_under_new_name(self):
        self.assert_change(
            "CREATE DOMAIN probe_type AS integer CHECK (VALUE > 0);\n"
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "ALTER DOMAIN probe_type RENAME TO renamed;\n"
            "ALTER TABLE recipe ADD COLUMN probe_value renamed; END$$;\n",
            CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_constrained_domain_in_execute_string_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "EXECUTE 'ALTER TABLE recipe ADD COLUMN weight pos_int'; END$$;\n",
            CONSTRAINED_DOMAIN, line=2,
        )

    def test_add_column_of_constrained_domain_with_volatile_default_reports_both_forms(self):
        self.assert_files({"0001_base.sql": BASE, "0002_change.sql":
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int DEFAULT (random() * 10 + 1)::integer;\n"
        }, [("0002_change.sql", 2, form) for form in (DEFAULT, CONSTRAINED_DOMAIN)])

    def test_body_created_constrained_domain_counts_only_inside_body(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int; END$$;\n"
            "ALTER TABLE recipe ADD COLUMN w2 pos_int;\n", CONSTRAINED_DOMAIN,
        )

    def test_add_column_of_not_null_domain_fails(self):
        self.assert_change(
            "CREATE DOMAIN required_int AS INTEGER NOT NULL;\n"
            "ALTER TABLE recipe ADD COLUMN weight required_int;\n", CONSTRAINED_DOMAIN, line=2,
        )

    def test_alter_column_type_to_constrained_domain_reports_only_type_change(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TABLE recipe ALTER COLUMN note TYPE pos_int;\n", TYPE, line=2,
        )

    def test_add_column_of_domain_over_constrained_domain_fails(self):
        self.assert_change(
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "CREATE DOMAIN weight_type AS pos_int;\n"
            "ALTER TABLE recipe ADD COLUMN weight weight_type;\n", CONSTRAINED_DOMAIN, line=3,
        )

    def assert_late_base_constraint(self, change):
        domains = "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n"
        column = "ALTER TABLE recipe ADD COLUMN w d2;\n"
        for later_file in (False, True):
            with self.subTest(later_file=later_file):
                files = {"0001_base.sql": BASE, "0002_change.sql": domains + change + column}
                line = len((domains + change).splitlines()) + 1
                name = "0002_change.sql"
                if later_file:
                    files["0002_change.sql"] = domains + change
                    files["0003_column.sql"] = column
                    name, line = "0003_column.sql", 1
                self.assert_files(files, [(name, line, CONSTRAINED_DOMAIN)])

    def test_child_domain_after_base_add_check_fails(self):
        self.assert_late_base_constraint("ALTER DOMAIN d1 ADD CHECK (VALUE > 0) NOT VALID;\n")

    def test_child_domain_after_base_set_not_null_fails(self):
        self.assert_late_base_constraint("ALTER DOMAIN d1 SET NOT NULL;\n")

    def test_child_domain_after_renamed_base_add_check_fails(self):
        self.assert_late_base_constraint(
            "ALTER DOMAIN d1 RENAME TO d3;\n"
            "ALTER DOMAIN d3 ADD CHECK (VALUE > 0) NOT VALID;\n"
        )

    def test_domain_chain_moves_keep_late_constraints(self):
        for kind in ("DOMAIN", "TYPE"):
            for moved in ("d1", "d2", "d3"):
                for action, target in (("RENAME TO renamed", "renamed"),
                                       ("SET SCHEMA staging", f"staging.{moved}")):
                    with self.subTest(kind=kind, moved=moved, action=action):
                        base = target if moved == "d1" else "d1"
                        column = target if moved == "d3" else "d3"
                        self.assert_files({"0001_base.sql": BASE +
                            "CREATE SCHEMA staging;\nCREATE DOMAIN d1 AS int;\n"
                            "CREATE DOMAIN d2 AS d1;\nCREATE DOMAIN d3 AS d2;\n",
                            "0002_change.sql": f"ALTER {kind} {moved} {action};\n"
                            f"ALTER DOMAIN {base} ADD CHECK (VALUE > 0) NOT VALID;\n",
                            "0003_column.sql": f"ALTER TABLE recipe ADD COLUMN w {column};\n"
                        }, [("0003_column.sql", 1, CONSTRAINED_DOMAIN)])

    def test_drop_domain_removes_base_link(self):
        self.assert_change(
            "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n"
            "DROP DOMAIN d2;\nCREATE TYPE d2 AS ENUM ('a');\n"
            "ALTER DOMAIN d1 ADD CHECK (VALUE > 0) NOT VALID;\n"
            "ALTER TABLE recipe ADD COLUMN w d2;\n"
        )

    def test_body_domain_chain_counts_in_execute_only_inside_body(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n"
            "ALTER DOMAIN d1 ADD CHECK (VALUE > 0) NOT VALID;\n"
            "EXECUTE 'ALTER TABLE recipe ADD COLUMN w d2'; END$$;\n"
            "CREATE DOMAIN d1 AS int CHECK (VALUE > 0);\n"
            "CREATE TYPE d2 AS ENUM ('a');\n"
            "ALTER TABLE recipe ADD COLUMN w2 d2;\n", CONSTRAINED_DOMAIN,
        )

    def test_body_domain_move_keeps_file_base_links(self):
        for moved in ("d1", "d2"):
            with self.subTest(moved=moved):
                base = "renamed" if moved == "d1" else "d1"
                column = "renamed" if moved == "d2" else "d2"
                self.assert_files({"0001_base.sql": BASE, "0002_change.sql":
                    "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n"
                    "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
                    f"ALTER DOMAIN {moved} RENAME TO renamed;\n"
                    f"ALTER DOMAIN {base} ADD CHECK (VALUE > 0) NOT VALID;\n"
                    f"ALTER TABLE recipe ADD COLUMN w {column}; END$$;\n"
                    "ALTER TABLE recipe ADD COLUMN w2 d2;\n"
                    "ALTER DOMAIN d1 ADD CHECK (VALUE > 0) NOT VALID;\n"
                    "ALTER TABLE recipe ADD COLUMN w3 d2;\n"
                }, [("0002_change.sql", line, CONSTRAINED_DOMAIN) for line in (3, 9)])

    def test_body_conditional_domain_rename_keeps_original_base_link(self):
        self.assert_files({"0001_base.sql":
            "CREATE TABLE probe (id int);\n"
            "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n",
            "0002_change.sql":
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$\n"
            "BEGIN\nIF false THEN\nALTER DOMAIN d1 RENAME TO d3;\nEND IF;\n"
            "ALTER DOMAIN d1 ADD CHECK (VALUE > 0) NOT VALID;\n"
            "ALTER TABLE probe ADD COLUMN w d2;\nEND;\n$$;\nSELECT f();\n"
            "ALTER TABLE probe ADD COLUMN w2 d2;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_body_conditional_domain_recreate_keeps_original_base_link(self):
        self.assert_files({"0001_base.sql":
            "CREATE TABLE probe (id int);\n"
            "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n",
            "0002_change.sql":
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$\n"
            "BEGIN\nIF false THEN\nDROP DOMAIN d2;\nCREATE DOMAIN d2 AS int;\nEND IF;\n"
            "ALTER DOMAIN d1 ADD CHECK (VALUE > 0) NOT VALID;\n"
            "ALTER TABLE probe ADD COLUMN w d2;\nEND;\n$$;\nSELECT f();\n"
            "ALTER TABLE probe ADD COLUMN w2 d2;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_body_domain_base_cycle_checks_all_possible_bases(self):
        for constraint in ("", "ALTER DOMAIN d3 ADD CHECK (VALUE > 0) NOT VALID;\n"):
            with self.subTest(constraint=constraint):
                self.assert_files({"0001_base.sql": BASE +
                    "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n",
                    "0002_change.sql":
                    "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
                    "IF false THEN\nALTER DOMAIN d1 RENAME TO d3;\n"
                    "CREATE DOMAIN d1 AS d2;\n" + constraint + "END IF;\n"
                    "ALTER TABLE recipe ADD COLUMN w d1; END$$;\n"
                }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)] if constraint else [])

    def test_child_domain_does_not_inherit_later_volatile_default(self):
        self.assert_change(
            "CREATE DOMAIN d1 AS int;\nCREATE DOMAIN d2 AS d1;\n"
            "ALTER DOMAIN d1 SET DEFAULT random();\n"
            "ALTER TABLE recipe ADD COLUMN w d2;\n"
        )

    def test_add_column_of_constrained_domain_from_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight pos_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_add_check_not_valid_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER;\n"
            "ALTER DOMAIN pos_int ADD CHECK (VALUE > 0) NOT VALID;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight pos_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_add_named_check_not_valid_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER;\n"
            "ALTER DOMAIN pos_int ADD CONSTRAINT positive CHECK (VALUE > 0) NOT VALID;\n",
            "0002_change.sql": "ALTER TABLE recipe ADD COLUMN weight pos_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_set_not_null_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN required_int AS INTEGER;\n"
            "ALTER DOMAIN required_int SET NOT NULL;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight required_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_add_not_null_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN required_int AS INTEGER;\n"
            "ALTER DOMAIN required_int ADD NOT NULL;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight required_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_add_named_not_null_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN required_int AS INTEGER;\n"
            "ALTER DOMAIN required_int ADD CONSTRAINT c NOT NULL;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight required_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_body_domain_add_not_null_counts_only_inside_body(self):
        for name in ("", "CONSTRAINT c "):
            with self.subTest(name=name):
                self.assert_files({"0001_base.sql": BASE +
                    "CREATE DOMAIN required_int AS INTEGER;\n", "0002_change.sql":
                    "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
                    f"ALTER DOMAIN required_int ADD {name}NOT NULL;\n"
                    "ALTER TABLE recipe ADD COLUMN weight required_int; END$$;\n"
                    "ALTER TABLE recipe ADD COLUMN w2 required_int;\n"
                }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_domain_add_not_null_needs_no_not_valid(self):
        for name in ("", "CONSTRAINT c "):
            with self.subTest(name=name):
                self.assert_files({"0001_base.sql": BASE +
                    "CREATE DOMAIN required_int AS INTEGER;\n", "0002_change.sql":
                    f"ALTER DOMAIN required_int ADD {name}NOT NULL;\n"
                })

    def test_add_column_after_alter_domain_rename_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER DOMAIN pos_int RENAME TO weight_type;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight weight_type;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_alter_domain_set_schema_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE SCHEMA staging;\n"
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER DOMAIN pos_int SET SCHEMA staging;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight staging.pos_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_alter_type_rename_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TYPE pos_int RENAME TO weight_type;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight weight_type;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_alter_type_set_schema_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE SCHEMA staging;\n"
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "ALTER TYPE pos_int SET SCHEMA staging;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight staging.pos_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_drop_constraint_keeps_form(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER CONSTRAINT positive CHECK (VALUE > 0);\n"
            "ALTER DOMAIN pos_int DROP CONSTRAINT positive;\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN weight pos_int;\n"
        }, [("0002_change.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_drop_not_null_in_earlier_file_keeps_form(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER NOT NULL;\n",
            "0002_drop.sql": "ALTER DOMAIN pos_int DROP NOT NULL;\n",
            "0003_column.sql": "ALTER TABLE recipe ADD COLUMN w pos_int;\n"
        }, [("0003_column.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_after_domain_drop_constraint_in_earlier_file_keeps_form(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER CONSTRAINT positive CHECK (VALUE > 0);\n",
            "0002_drop.sql": "ALTER DOMAIN pos_int DROP CONSTRAINT positive;\n",
            "0003_column.sql": "ALTER TABLE recipe ADD COLUMN w pos_int;\n"
        }, [("0003_column.sql", 1, CONSTRAINED_DOMAIN)])

    def test_add_column_of_domain_with_volatile_default_fails(self):
        self.assert_change(
            "CREATE DOMAIN row_key AS UUID DEFAULT gen_random_uuid();\n"
            "ALTER TABLE recipe ADD COLUMN token row_key;\n", DEFAULT, line=2,
        )

    def test_add_column_of_domain_over_volatile_default_domain_fails(self):
        self.assert_change(
            "CREATE DOMAIN row_key AS UUID DEFAULT gen_random_uuid();\n"
            "CREATE DOMAIN token_type AS row_key;\n"
            "ALTER TABLE recipe ADD COLUMN token token_type;\n", DEFAULT, line=3,
        )

    def test_add_column_after_domain_set_volatile_default_in_earlier_file_fails(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN row_key AS UUID;\n"
            "ALTER DOMAIN row_key SET DEFAULT gen_random_uuid();\n", "0002_change.sql":
            "ALTER TABLE recipe ADD COLUMN token row_key;\n"
        }, [("0002_change.sql", 1, DEFAULT)])

    def test_add_column_of_unconstrained_domain_passes(self):
        self.assert_change(
            "CREATE DOMAIN weight_type AS INTEGER;\n"
            "ALTER TABLE recipe ADD COLUMN weight weight_type;\n"
        )

    def test_add_column_after_domain_drop_default_in_earlier_file_keeps_volatile_form(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN row_key AS UUID DEFAULT gen_random_uuid();\n",
            "0002_drop.sql": "ALTER DOMAIN row_key DROP DEFAULT;\n",
            "0003_column.sql": "ALTER TABLE recipe ADD COLUMN token row_key;\n"
        }, [("0003_column.sql", 1, DEFAULT)])

    def test_add_column_of_domain_with_only_null_passes(self):
        self.assert_change(
            "CREATE DOMAIN weight_type AS INTEGER NULL;\n"
            "ALTER TABLE recipe ADD COLUMN weight weight_type;\n"
        )

    def test_add_column_of_domain_with_non_volatile_default_passes(self):
        self.assert_change(
            "CREATE DOMAIN created_at AS TIMESTAMPTZ DEFAULT now();\n"
            "ALTER TABLE recipe ADD COLUMN recorded_at created_at;\n"
        )

    def test_add_column_of_domain_with_only_collate_passes(self):
        self.assert_change(
            'CREATE DOMAIN label AS TEXT COLLATE "C";\n'
            "ALTER TABLE recipe ADD COLUMN label_text label;\n"
        )

    def test_column_default_overrides_volatile_domain_default(self):
        self.assert_change(
            "CREATE DOMAIN row_key AS UUID DEFAULT gen_random_uuid();\n"
            "ALTER TABLE recipe ADD COLUMN token row_key "
            "DEFAULT '00000000-0000-0000-0000-000000000001'::uuid;\n"
        )

    def test_domain_default_overrides_volatile_base_domain_default(self):
        self.assert_change(
            "CREATE DOMAIN row_key AS UUID DEFAULT gen_random_uuid();\n"
            "CREATE DOMAIN token_type AS row_key "
            "DEFAULT '00000000-0000-0000-0000-000000000001'::uuid;\n"
            "ALTER TABLE recipe ADD COLUMN token token_type;\n"
        )

    def test_domain_default_is_not_null_expression_is_not_a_constraint(self):
        self.assert_change(
            "CREATE DOMAIN flag AS BOOLEAN DEFAULT 1 IS NOT NULL;\n"
            "ALTER TABLE recipe ADD COLUMN enabled flag;\n"
        )

    def test_add_column_of_constrained_domain_on_same_file_table_passes(self):
        self.assert_files({"0001_new.sql":
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n"
            "CREATE TABLE recipe (note TEXT);\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int;\n"
        })

    def test_add_column_of_volatile_default_domain_on_same_file_table_passes(self):
        self.assert_files({"0001_new.sql":
            "CREATE DOMAIN row_key AS UUID DEFAULT gen_random_uuid();\n"
            "CREATE TABLE recipe (note TEXT);\n"
            "ALTER TABLE recipe ADD COLUMN token row_key;\n"
        })

    def test_drop_and_recreate_domain_without_constraint_passes(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);\n", "0002_change.sql":
            "DROP DOMAIN pos_int;\n"
            "CREATE DOMAIN pos_int AS INTEGER;\n"
            "ALTER TABLE recipe ADD COLUMN weight pos_int;\n"
        })

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
