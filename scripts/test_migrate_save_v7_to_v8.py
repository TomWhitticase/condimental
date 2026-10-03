#!/usr/bin/env python3
"""Pins `scripts/migrate-save-v7-to-v8.py` to the file it is proved against.

`sim_core/tests/fixtures/save_v7_two_lines.ron` is a real version-7 save,
written by the last version-7 build (the ketchup line and the cooked mustard
line, both mid-craft). `save_v7_two_lines.migrated.ron` is what the script
turns it into, and `sim_core/tests/save_migration.rs` loads THAT file into the
current build and runs it. This check is the other half: the script must
still produce exactly that file, so an edit to the script cannot leave the
Rust test proving something about output the script no longer writes.

Run by `.github/workflows/invariants.yml`.
"""

import importlib.util
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "sim_core" / "tests" / "fixtures"

spec = importlib.util.spec_from_file_location(
    "migrate_v7_to_v8", ROOT / "scripts" / "migrate-save-v7-to-v8.py"
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

_, migrated = module.migrate(FIXTURES / "save_v7_two_lines.ron", apply=False)
expected = (FIXTURES / "save_v7_two_lines.migrated.ron").read_text()
if migrated != expected:
    print(
        "migrate-save-v7-to-v8.py no longer produces save_v7_two_lines.migrated.ron — "
        "regenerate the fixture with --apply on a copy and re-run cargo test -p sim_core "
        "--test save_migration, or fix the script"
    )
    sys.exit(1)
print("MIGRATE_V7_V8_OK: the script reproduces the migrated fixture byte for byte")
