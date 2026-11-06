"""Check migration rewrite rules with temporary SQL files and no database."""

import contextlib
import importlib.util
import io
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SCRIPT = pathlib.Path(__file__).with_name("migration_rewrite.py")
MIGRATIONS = pathlib.Path(__file__).resolve().parents[2] / "migrations"
BASE = "CREATE TABLE recipe (note TEXT, other TEXT);\n"
CHECK = "ALTER TABLE recipe ADD CONSTRAINT note_nn CHECK (note IS NOT NULL) NOT VALID"
VALIDATED_CHECK = CHECK + ";\nALTER TABLE recipe VALIDATE CONSTRAINT note_nn;\n"
SET_NOT_NULL = "ALTER TABLE recipe ALTER COLUMN note SET NOT NULL;\n"
TYPE = "column type change"
DEFAULT = "volatile default"
NOT_NULL = "set not null without valid check"


class MigrationRewriteTestCase(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "Task 6.2 must add migration_rewrite.py")
        spec = importlib.util.spec_from_file_location("migration_rewrite", SCRIPT)
        self.rewrite = importlib.util.module_from_spec(spec)
        with mock.patch.dict(sys.modules, {spec.name: self.rewrite}):
            spec.loader.exec_module(self.rewrite)
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = pathlib.Path(temporary.name)

    def write_files(self, files):
        for name, sql in files.items():
            (self.directory / name).write_text(sql, encoding="utf-8", newline="\n")

    def run_cli(self, directory=None):
        return subprocess.run(
            [sys.executable, "-I", "-B", str(SCRIPT), str(directory or self.directory)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
            timeout=30,
        )

    def diagnostics(self, output):
        diagnostics = []
        for line in output.splitlines():
            match = self.assert_diagnostic(line)
            diagnostics.append((pathlib.Path(match[0]).name, int(match[1]), match[2]))
        return diagnostics

    def assert_diagnostic(self, line):
        parts = line.rsplit(":", 2)
        self.assertEqual(len(parts), 3, line)
        self.assertTrue(parts[1].isdigit(), line)
        self.assertTrue(parts[2].startswith(" "), line)
        return parts[0], parts[1], parts[2][1:]

    def assert_files(self, files, expected=(), exit_code=None):
        self.write_files(files)
        result = self.run_with_exempt({})
        wanted = exit_code if exit_code is not None else int(bool(expected))
        self.assertEqual(result.returncode, wanted, result.stdout + result.stderr)
        if wanted == 1:
            self.assertEqual(self.diagnostics(result.stdout), list(expected))
        return result

    def assert_change(self, sql, form=None, line=1):
        expected = [("0002_change.sql", line, form)] if form else []
        return self.assert_files(
            {"0001_base.sql": BASE, "0002_change.sql": sql}, expected
        )

    def assert_check_change(self, change, form=None, check=VALIDATED_CHECK):
        expected = [("0003_change.sql", 1, form)] if form else []
        return self.assert_files(
            {"0001_base.sql": BASE, "0002_check.sql": check, "0003_change.sql": change},
            expected,
        )

    def run_with_exempt(self, exemptions):
        """Call the CLI main function with a temporary exemption map."""
        stdout, stderr = io.StringIO(), io.StringIO()
        with (
            mock.patch.object(self.rewrite, "EXEMPT", exemptions),
            mock.patch.object(sys, "argv", [str(SCRIPT), str(self.directory)]),
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            try:
                code = self.rewrite.main()
            except SystemExit as error:
                code = error.code
        return subprocess.CompletedProcess([], code or 0, stdout.getvalue(), stderr.getvalue())


class MigrationRewriteTests(MigrationRewriteTestCase):
    def test_column_type_change_fails(self):
        self.assert_files({
            "0001_base.sql": "CREATE TABLE audit_log (action INTEGER);\n",
            "0002_change.sql": "ALTER TABLE audit_log ALTER COLUMN action TYPE TEXT;\n",
        }, [("0002_change.sql", 1, TYPE)])

    def test_column_type_change_without_column_keyword_fails(self):
        self.assert_change("ALTER TABLE recipe ALTER note TYPE TEXT;\n", TYPE)

    def test_set_data_type_fails(self):
        self.assert_change("ALTER TABLE recipe ALTER COLUMN note SET DATA TYPE TEXT;\n", TYPE)

    def test_schema_qualified_table_fails(self):
        self.assert_change("ALTER TABLE public.recipe ALTER COLUMN note TYPE TEXT;\n", TYPE)

    def test_temporary_table_does_not_exempt_public_table(self):
        for declaration in ("TEMP TABLE recipe", "TEMPORARY TABLE recipe", "TABLE pg_temp.recipe"):
            with self.subTest(declaration=declaration):
                self.assert_files(
                    {
                        "0001_base.sql": "CREATE TABLE public.recipe (note TEXT);\n",
                        "0002_change.sql": f"CREATE {declaration} (note TEXT);\n"
                            "ALTER TABLE public.recipe ALTER COLUMN note TYPE INTEGER USING 1;\n",
                    },
                    [("0002_change.sql", 2, TYPE)],
                )

    def test_unqualified_alter_resolves_to_temporary_table(self):
        for declaration in ("TEMP TABLE recipe", "TEMPORARY TABLE recipe", "TABLE pg_temp.recipe"):
            with self.subTest(declaration=declaration):
                self.assert_change(
                    f"CREATE {declaration} (note TEXT);\n"
                    "ALTER TABLE recipe ALTER COLUMN note TYPE INTEGER USING 1;\n"
                )

    def test_public_and_unqualified_names_share_exemption(self):
        for created, altered in (("public.x", "x"), ("x", "public.x")):
            with self.subTest(created=created, altered=altered):
                self.assert_files({"0001_base.sql":
                    f"CREATE TABLE {created} (note TEXT);\n"
                    f"ALTER TABLE {altered} ALTER COLUMN note TYPE INTEGER USING 1;\n"
                })

    def test_other_schema_does_not_exempt_public_table(self):
        for target in ("recipe", "public.recipe"):
            with self.subTest(target=target):
                self.assert_change(
                    "CREATE SCHEMA staging;\nCREATE TABLE staging.recipe (note TEXT);\n"
                    f"ALTER TABLE {target} ALTER COLUMN note TYPE INTEGER USING 1;\n",
                    TYPE, line=3,
                )

    def test_check_in_another_schema_does_not_prove_not_null(self):
        self.assert_check_change(
            SET_NOT_NULL, NOT_NULL,
            check="CREATE SCHEMA staging;\nCREATE TABLE staging.recipe (note TEXT, "
                  "CONSTRAINT note_nn CHECK (note IS NOT NULL));\n",
        )

    def test_renamed_temporary_table_keeps_its_namespace(self):
        self.assert_change(
            "CREATE TEMP TABLE scratch (note TEXT);\n"
            "ALTER TABLE scratch RENAME TO recipe;\n"
            "ALTER TABLE recipe ALTER COLUMN note TYPE INTEGER USING 1;\n"
            "ALTER TABLE public.recipe ALTER COLUMN note TYPE INTEGER USING 1;\n",
            TYPE, line=4,
        )

    def test_drop_temporary_table_restores_public_resolution(self):
        self.assert_change(
            "CREATE TEMP TABLE recipe (note TEXT);\nDROP TABLE recipe;\n"
            "ALTER TABLE recipe ALTER COLUMN note TYPE INTEGER USING 1;\n",
            TYPE, line=3,
        )

    def test_drop_temporary_table_preserves_public_check(self):
        self.assert_check_change(
            "CREATE TEMP TABLE recipe (note TEXT);\nDROP TABLE recipe;\n" + SET_NOT_NULL
        )

    def test_only_table_fails(self):
        self.assert_change("ALTER TABLE ONLY recipe ALTER COLUMN note TYPE TEXT;\n", TYPE)

    def test_if_exists_table_fails(self):
        self.assert_change("ALTER TABLE IF EXISTS recipe ALTER COLUMN note TYPE TEXT;\n", TYPE)

    def test_lowercase_and_multiline_statement_reports_start_line(self):
        self.assert_change(
            "-- Comment before the statement.\n\nalter table recipe\n"
            "  alter column note\n  set data type text;\n",
            TYPE,
            line=3,
        )

    def test_uuid_default_fails(self):
        self.assert_change(
            "ALTER TABLE recipe ADD COLUMN token UUID DEFAULT gen_random_uuid();\n", DEFAULT
        )

    def test_nextval_default_fails(self):
        self.assert_files({
            "0001_base.sql": BASE + "CREATE SEQUENCE recipe_seq;\n",
            "0002_change.sql":
                "ALTER TABLE recipe ADD COLUMN seq BIGINT DEFAULT nextval('recipe_seq');\n",
        }, [("0002_change.sql", 1, DEFAULT)])

    def test_serial_types_fail(self):
        for type_name in ("serial", "smallserial", "bigserial", "serial2", "serial4", "serial8"):
            for prefix in ("ADD COLUMN", "ADD", "ADD COLUMN IF NOT EXISTS"):
                with self.subTest(type_name=type_name, prefix=prefix):
                    self.assert_change(
                        f"ALTER TABLE recipe {prefix} s {type_name};\n", DEFAULT
                    )

    def test_identity_columns_fail(self):
        for generation in ("ALWAYS", "BY DEFAULT"):
            with self.subTest(generation=generation):
                self.assert_change(
                    f"ALTER TABLE recipe ADD COLUMN s bigint GENERATED {generation} AS IDENTITY;\n",
                    DEFAULT,
                )

    def test_new_table_permits_serial_and_identity_columns(self):
        for definition in (
            "serial", "smallserial", "bigserial", "serial2", "serial4", "serial8",
            "bigint GENERATED ALWAYS AS IDENTITY",
            "bigint GENERATED BY DEFAULT AS IDENTITY",
        ):
            with self.subTest(definition=definition):
                self.assert_files({"0001_base.sql": BASE +
                    f"ALTER TABLE recipe ADD COLUMN s {definition};\n"
                })

    def test_unknown_default_function_fails(self):
        self.assert_change(
            "ALTER TABLE recipe ADD COLUMN token TEXT DEFAULT custom_value();\n", DEFAULT
        )

    def test_add_without_column_keyword_fails(self):
        self.assert_change("ALTER TABLE recipe ADD token UUID DEFAULT gen_random_uuid();\n", DEFAULT)

    def test_now_default_passes(self):
        self.assert_change("ALTER TABLE recipe ADD COLUMN created_at TIMESTAMPTZ DEFAULT now();\n")

    def test_null_tests_in_default_do_not_hide_volatile_calls(self):
        for predicate in ("1 IS NOT NULL", "1 IS NULL"):
            with self.subTest(predicate=predicate):
                self.assert_change(
                    "ALTER TABLE recipe ADD COLUMN u uuid DEFAULT "
                    f"CASE WHEN {predicate} THEN gen_random_uuid() END;\n", DEFAULT,
                )

    def test_null_case_result_does_not_hide_volatile_call(self):
        self.assert_change(
            "ALTER TABLE recipe ADD COLUMN u uuid DEFAULT "
            "CASE WHEN TRUE THEN NULL ELSE gen_random_uuid() END;\n", DEFAULT,
        )

    def test_null_predicate_precedes_column_constraint(self):
        for predicate in ("1 IS NULL", "1 IS NOT NULL"):
            with self.subTest(predicate=predicate):
                _, tokens = next(self.rewrite.statements(f"{predicate} NOT NULL"))
                self.assertEqual(
                    self.rewrite.default_expression(tokens), predicate.lower().split(),
                )

    def test_default_ends_before_column_constraints(self):
        for constraint in (
            "NOT NULL", "NULL", "CONSTRAINT c CHECK (length(c) > 0)",
            "CHECK (length(c) > 0)", "UNIQUE", "PRIMARY KEY",
            "REFERENCES other_table(c)", 'COLLATE "C"',
            "GENERATED ALWAYS AS IDENTITY",
        ):
            with self.subTest(constraint=constraint):
                _, tokens = next(self.rewrite.statements(f"'x' {constraint}"))
                self.assertEqual(self.rewrite.default_expression(tokens), ["'x'"])

    def test_cast_type_modifiers_are_not_function_calls(self):
        for literal, type_name in (
            ("'x'", "character varying(5)"), ("1", "numeric(10,2)"),
            ("B'101'", "bit varying(3)"),
            ("'2000-01-01 00:00:00+00'", "timestamp(3) with time zone"),
            ("'12:00:00'", "time(3)"),
        ):
            for expression in (f"{literal}::{type_name}", f"CAST({literal} AS {type_name})"):
                with self.subTest(expression=expression):
                    self.assert_change(
                        f"ALTER TABLE recipe ADD COLUMN c TEXT DEFAULT {expression};\n"
                    )

    def test_cast_keeps_volatile_function_detection(self):
        for expression in (
            "custom_value()::character varying(5)",
            "CAST(custom_value() AS character varying(5))",
            "'x'::character varying(5) || custom_value()",
            "varying(5)",
        ):
            with self.subTest(expression=expression):
                self.assert_change(
                    f"ALTER TABLE recipe ADD COLUMN c TEXT DEFAULT {expression};\n", DEFAULT,
                )

    def test_null_literal_default_passes(self):
        self.assert_change("ALTER TABLE recipe ADD COLUMN c TEXT DEFAULT NULL;\n")

    def test_literal_defaults_pass(self):
        self.assert_change(
            "ALTER TABLE recipe ADD COLUMN label TEXT DEFAULT 'a,b; it''s literal',\n"
            " ADD COLUMN amount NUMERIC(10, 2) DEFAULT 1.25,\n"
            " ADD COLUMN enabled BOOLEAN DEFAULT TRUE;\n"
        )

    def test_set_not_null_without_check_fails(self):
        self.assert_change(SET_NOT_NULL, NOT_NULL)

    def test_do_with_direct_ddl_fails(self):
        self.assert_change(
            "\nDO $$ BEGIN\n ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\nEND $$;\n",
            "DO block",
            line=2,
        )

    def test_do_with_execute_fails(self):
        self.assert_change(
            "DO $block$ BEGIN\n"
            " EXECUTE 'ALTER TABLE recipe ALTER COLUMN note TYPE TEXT';\nEND $block$;\n",
            "DO block",
        )

    def test_do_without_ddl_also_fails(self):
        self.assert_change("DO $$ BEGIN NULL; END $$;\n", "DO block")

    def test_new_table_in_same_file_permits_all_three_forms(self):
        self.assert_files({"0001_base.sql": BASE + (
            "ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\n"
            "ALTER TABLE recipe ADD COLUMN token UUID DEFAULT gen_random_uuid();\n"
            + SET_NOT_NULL
        )})

    def test_new_table_does_not_exempt_do(self):
        self.assert_files({"0001_base.sql": BASE + "DO $$ BEGIN NULL; END $$;\n"},
                          [("0001_base.sql", 2, "DO block")])

    def test_conditional_create_does_not_exempt_old_table(self):
        self.assert_change(
            "CREATE TABLE IF NOT EXISTS recipe (note TEXT);\n"
            "ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\n",
            TYPE,
            line=2,
        )

    def test_validated_check_in_earlier_file_passes(self):
        self.assert_check_change(SET_NOT_NULL)

    def test_validation_in_separate_file_passes_in_name_order(self):
        self.assert_files({
            "0004_change.sql": SET_NOT_NULL,
            "0003_validate.sql": "ALTER TABLE recipe VALIDATE CONSTRAINT note_nn;\n",
            "0002_check.sql": CHECK + ";\n",
            "0001_base.sql": BASE,
        })

    def test_dropped_unnamed_check_after_name_collision_fails(self):
        self.assert_files({
            "0001_base.sql":
                "CREATE TABLE recipe (note TEXT CHECK (length(note) < 10));\n"
                "ALTER TABLE recipe ADD CHECK (note IS NOT NULL);\n",
            "0002_change.sql":
                "ALTER TABLE recipe DROP CONSTRAINT recipe_note_check1;\n" + SET_NOT_NULL,
        }, [("0002_change.sql", 2, NOT_NULL)])

    def test_unnamed_check_does_not_prove_not_null(self):
        for declaration in (
            "CREATE TABLE recipe (note TEXT CHECK (note IS NOT NULL));\n",
            "CREATE TABLE recipe (note TEXT, CHECK (note IS NOT NULL));\n",
            BASE + "ALTER TABLE recipe ADD CHECK (note IS NOT NULL);\n",
        ):
            with self.subTest(declaration=declaration):
                self.assert_files(
                    {"0001_base.sql": declaration, "0002_change.sql": SET_NOT_NULL},
                    [("0002_change.sql", 1, NOT_NULL)],
                )

    def test_long_unnamed_check_name_does_not_prove_not_null(self):
        table = "recipe_" + "x" * 50
        self.assert_files({
            "0001_base.sql": f"CREATE TABLE {table} (note TEXT);\n"
                f"ALTER TABLE {table} ADD CHECK (note IS NOT NULL);\n",
            "0002_change.sql": f"ALTER TABLE {table} ALTER COLUMN note SET NOT NULL;\n",
        }, [("0002_change.sql", 1, NOT_NULL)])

    def test_named_check_after_unnamed_check_passes(self):
        self.assert_files({
            "0001_base.sql":
                "CREATE TABLE recipe (note TEXT CHECK (length(note) < 10));\n"
                + VALIDATED_CHECK,
            "0002_change.sql": SET_NOT_NULL,
        })

    def test_unvalidated_check_fails(self):
        self.assert_check_change(SET_NOT_NULL, NOT_NULL, check=CHECK + ";\n")

    def test_dropped_check_in_later_file_fails(self):
        self.assert_files({
            "0001_base.sql": BASE + VALIDATED_CHECK,
            "0002_drop.sql": "ALTER TABLE recipe DROP CONSTRAINT note_nn;\n",
            "0003_change.sql": SET_NOT_NULL,
        }, [("0003_change.sql", 1, NOT_NULL)])

    def test_drop_check_in_same_statement_fails(self):
        self.assert_check_change(
            "ALTER TABLE recipe ALTER COLUMN note SET NOT NULL, DROP CONSTRAINT note_nn;\n",
            NOT_NULL,
        )

    def test_renamed_valid_check_passes(self):
        self.assert_check_change(
            "ALTER TABLE recipe RENAME CONSTRAINT note_nn TO note_present;\n" + SET_NOT_NULL
        )

    def test_validate_renamed_check_passes(self):
        self.assert_check_change(
            "ALTER TABLE recipe RENAME CONSTRAINT note_nn TO note_present;\n"
            "ALTER TABLE recipe VALIDATE CONSTRAINT note_present;\n" + SET_NOT_NULL,
            check=CHECK + ";\n",
        )

    def test_check_on_other_column_fails(self):
        self.assert_check_change(
            SET_NOT_NULL, NOT_NULL,
            check=VALIDATED_CHECK.replace("note", "other"),
        )

    def test_check_on_other_table_fails(self):
        self.assert_check_change(
            SET_NOT_NULL, NOT_NULL,
            check="CREATE TABLE other_recipe (note TEXT, CHECK (note IS NOT NULL));\n",
        )

    def test_schema_qualified_check_matches_unqualified_table(self):
        self.assert_check_change(SET_NOT_NULL, check=VALIDATED_CHECK.replace("recipe", "public.recipe"))

    def test_renamed_column_keeps_valid_check(self):
        self.assert_check_change(
            "ALTER TABLE recipe RENAME COLUMN note TO description;\n"
            "ALTER TABLE recipe ALTER COLUMN description SET NOT NULL;\n"
        )

    def test_renamed_table_keeps_valid_check(self):
        self.assert_check_change(
            "ALTER TABLE recipe RENAME TO saved_recipe;\n"
            "ALTER TABLE saved_recipe ALTER COLUMN note SET NOT NULL;\n"
        )

    def test_dropped_column_removes_check_state(self):
        self.assert_files({
            "0001_base.sql": BASE + VALIDATED_CHECK,
            "0002_replace.sql":
                "ALTER TABLE recipe DROP COLUMN note;\nALTER TABLE recipe ADD COLUMN note TEXT;\n",
            "0003_change.sql": SET_NOT_NULL,
        }, [("0003_change.sql", 1, NOT_NULL)])

    def test_dropped_table_removes_check_state(self):
        self.assert_files({
            "0001_base.sql": BASE + VALIDATED_CHECK,
            "0002_replace.sql": "DROP TABLE recipe;\n" + BASE,
            "0003_change.sql": SET_NOT_NULL,
        }, [("0003_change.sql", 1, NOT_NULL)])

    def test_multiple_actions_report_each_form_at_statement_start(self):
        self.assert_files(
            {
                "0001_base.sql": BASE,
                "0002_change.sql":
                    "-- One statement with several actions.\nALTER TABLE recipe\n"
                    " ADD COLUMN amount NUMERIC(10, 2) DEFAULT 1.25,\n"
                    " ALTER COLUMN note TYPE TEXT,\n"
                    " ADD COLUMN token UUID DEFAULT gen_random_uuid(),\n"
                    " ALTER COLUMN other SET NOT NULL;\n",
            },
            [("0002_change.sql", 2, form) for form in (TYPE, DEFAULT, NOT_NULL)],
        )

    def assert_function_body(self, delimiter):
        self.assert_files({"0001_base.sql": BASE, "0002_change.sql":
            "CREATE FUNCTION example() RETURNS void LANGUAGE plpgsql\n"
            f"SET search_path = pg_catalog, public, pg_temp AS {delimiter}\nBEGIN\n"
            " ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\n"
            " ALTER TABLE recipe ADD COLUMN token UUID DEFAULT gen_random_uuid();\n"
            " ALTER TABLE recipe ALTER COLUMN note SET NOT NULL;\n"
            f"END\n{delimiter};\n"
        }, [("0002_change.sql", 1, form) for form in (TYPE, DEFAULT, NOT_NULL)])

    def test_dollar_quoted_function_body_fails(self):
        self.assert_function_body("$$")

    def test_named_dollar_quoted_function_body_fails(self):
        self.assert_function_body("$fn$")

    def test_called_function_body_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql\n"
            "AS $$BEGIN ALTER TABLE recipe ALTER COLUMN note TYPE varchar(5); END$$;\n"
            "SELECT f();\n", TYPE,
        )

    def test_called_procedure_body_fails(self):
        self.assert_change(
            "CREATE PROCEDURE p() LANGUAGE plpgsql\n"
            "AS $$BEGIN ALTER TABLE recipe ALTER COLUMN note TYPE varchar(5); END$$;\n"
            "CALL p();\n", TYPE,
        )

    def test_uncalled_function_body_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql\n"
            "AS $$BEGIN ALTER TABLE recipe ALTER COLUMN note TYPE varchar(5); END$$;\n",
            TYPE,
        )

    def test_replaced_function_with_single_quoted_body_fails(self):
        self.assert_change(
            "CREATE OR REPLACE FUNCTION f() RETURNS void LANGUAGE plpgsql\n"
            "AS 'BEGIN ALTER TABLE recipe ALTER COLUMN note TYPE TEXT; END';\n", TYPE,
        )

    def test_function_body_does_not_establish_check_proof(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            + VALIDATED_CHECK + "END$$;\n" + SET_NOT_NULL, NOT_NULL, line=5,
        )

    def test_function_body_preserves_new_table_exemption(self):
        self.assert_files({"0001_base.sql": BASE +
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql\n"
            "AS $$BEGIN ALTER TABLE recipe ALTER COLUMN note TYPE TEXT; END$$;\n"
        })

    def test_function_body_ignores_comments_and_plain_strings(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "/* ALTER TABLE recipe ALTER COLUMN note TYPE TEXT; */\n"
            "RAISE NOTICE 'ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;';\n"
            "END$$;\n"
        )

    def test_function_body_with_execute_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "EXECUTE 'ALTER TABLE recipe ALTER COLUMN note TYPE TEXT';\n"
            "END$$;\n", TYPE,
        )

    def test_function_body_with_on_conflict_do_nothing_passes(self):
        for prefix in ("BEGIN\n", "BEGIN\nNULL;\n"):
            with self.subTest(prefix=prefix):
                self.assert_change(
                    "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$\n"
                    + prefix
                    + "INSERT INTO recipe (note) VALUES ('x') ON CONFLICT DO NOTHING;\n"
                    "END$$;\n"
                )

    def test_function_body_with_on_conflict_do_update_passes(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "INSERT INTO item (id, note) VALUES (1, 'x')\n"
            "ON CONFLICT (id) DO UPDATE SET note = EXCLUDED.note;\n"
            "END$$;\n"
        )

    def test_function_body_with_constraint_begin_do_nothing_passes(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "INSERT INTO item (id) VALUES (1)\n"
            "ON CONFLICT ON CONSTRAINT begin DO NOTHING;\n"
            "END$$;\n"
        )

    def test_function_body_with_constraint_loop_do_update_passes(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "INSERT INTO item (id, note) VALUES (1, 'x')\n"
            "ON CONFLICT ON CONSTRAINT loop DO UPDATE SET note = EXCLUDED.note;\n"
            "END$$;\n"
        )

    def test_function_body_with_executed_do_fails(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "EXECUTE 'DO $block$ BEGIN NULL; END $block$';\n"
            "END$$;\n", "DO block",
        )

    def test_function_create_does_not_exempt_existing_table(self):
        self.assert_change(
            "CREATE FUNCTION f() RETURNS void LANGUAGE plpgsql AS $$BEGIN\n"
            "CREATE TABLE recipe (note TEXT);\n"
            "ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\nEND$$;\n", TYPE,
        )

    def test_statement_after_function_body_still_fails(self):
        self.assert_change(
            "CREATE FUNCTION example() RETURNS void LANGUAGE plpgsql\n"
            "SET search_path = pg_catalog, public, pg_temp AS $fn$\n"
            "BEGIN NULL; END\n$fn$;\n" + SET_NOT_NULL,
            NOT_NULL,
            line=5,
        )

    def test_forms_inside_comments_pass(self):
        self.assert_change(
            "-- ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\n"
            "/* ALTER TABLE recipe ADD COLUMN token UUID DEFAULT gen_random_uuid();\n"
            "ALTER TABLE recipe ALTER COLUMN note SET NOT NULL;\n"
            "DO $$ BEGIN NULL; END $$; */\nSELECT 1;\n"
        )

    def test_form_inside_string_literal_passes(self):
        self.assert_change(
            "SELECT 'ALTER TABLE recipe ALTER COLUMN note TYPE TEXT; DO $$ BEGIN NULL; END $$;';\n"
        )

    def test_unterminated_single_quote_exits_two(self):
        self.assert_files({"0001_broken.sql": "SELECT 'unterminated;\n"}, exit_code=2)

    def test_unterminated_dollar_quote_exits_two(self):
        self.assert_files({"0001_broken.sql": "DO $fn$ BEGIN NULL; END;\n"}, exit_code=2)

    def test_unreadable_utf8_file_exits_two(self):
        (self.directory / "0001_broken.sql").write_bytes(b"\xff\xfe\x00")
        result = self.run_cli()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_non_sql_files_do_not_count(self):
        self.assert_files({"0001_base.sql": BASE, "notes.txt": "SELECT 'unterminated;\n"})

    def test_violations_from_all_files_follow_name_order(self):
        self.assert_files({
            "0003_change.sql": SET_NOT_NULL,
            "0002_change.sql": "ALTER TABLE recipe ALTER COLUMN note TYPE TEXT;\n",
            "0001_base.sql": BASE,
        }, [("0002_change.sql", 1, TYPE), ("0003_change.sql", 1, NOT_NULL)])

    def test_matching_exemption_passes(self):
        self.write_files({"0001_do.sql": "DO $$ BEGIN NULL; END $$;\n"})
        result = self.run_with_exempt({("0001_do.sql", "DO block"): "ADR-0127"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_exemption_does_not_hide_another_form(self):
        self.write_files({
            "0001_base.sql": BASE,
            "0002_change.sql": "DO $$ BEGIN NULL; END $$;\n" + SET_NOT_NULL,
        })
        result = self.run_with_exempt({("0002_change.sql", "DO block"): "ADR-0127"})
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.diagnostics(result.stdout), [("0002_change.sql", 2, NOT_NULL)])

    def test_dead_exemption_fails(self):
        self.write_files({"0001_base.sql": BASE})
        result = self.run_with_exempt({("0001_base.sql", "DO block"): "ADR-0127"})
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        diagnostics = self.diagnostics(result.stdout)
        self.assertEqual(len(diagnostics), 1, result.stdout)
        name, line, form = diagnostics[0]
        self.assertEqual((name, form), ("0001_base.sql", "dead exemption"))
        self.assertGreaterEqual(line, 0)

    def test_shipped_migrations_pass(self):
        self.assertTrue(MIGRATIONS.is_dir(), str(MIGRATIONS))
        files = {path.name: path.read_text(encoding="utf-8") for path in MIGRATIONS.glob("*.sql")
                 if path.name[:4] in {f"{number:04d}" for number in range(1, 11)}}
        self.assertEqual({name[:4] for name in files}, {f"{number:04d}" for number in range(1, 11)})
        result = self.run_cli(MIGRATIONS)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

if __name__ == "__main__":
    unittest.main()
