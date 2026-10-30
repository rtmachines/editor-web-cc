"""Check the migration freeze of a major upgrade release without Docker."""

import importlib.util
import pathlib
import unittest


COMPOSE = "services:\n  db:\n    image: postgres:{version}-bookworm@sha256:{digest}\n"
DIGEST = "d" * 64
NEW_MIGRATION = ["migrations/0099_new_table.sql"]


def compose(version):
    return COMPOSE.format(version=version, digest=DIGEST)


class MigrationFreezeTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("migration_freeze.py")
        spec = importlib.util.spec_from_file_location("migration_freeze", path)
        self.freeze = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.freeze)

    def test_new_major_version_with_a_new_migration_fails(self):
        failures = self.freeze.check(compose("17.2"), compose("16.15"), NEW_MIGRATION)
        self.assertEqual(failures, NEW_MIGRATION)

    def test_new_major_version_without_a_migration_passes(self):
        self.assertEqual(self.freeze.check(compose("17.2"), compose("16.15"), []), [])

    def test_same_major_version_with_a_new_migration_passes(self):
        self.assertEqual(self.freeze.check(compose("16.16"), compose("16.15"), NEW_MIGRATION), [])


if __name__ == "__main__":
    unittest.main()
