#!/usr/bin/env python3
# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Migration lint: a migration leaves the previous minor's server working
(CLOACINA-T-1087).

During a rolling upgrade the server of the previous minor runs on the schema
that the new minor migrated. A migration of minor N leaves a server of minor
N-1 working, with no SQL error and no wrong result:

1. Add only: tables, nullable columns, columns with a default, indexes.
2. No drop or rename of a table or column that N-1 reads or writes. It goes
   in N+1, with a "Changed (breaking)" entry in CHANGELOG.md.
3. No NOT NULL without a default on a table N-1 writes; no SET NOT NULL.
4. No new unique index or constraint that N-1's writes can break.
5. No new meaning for a column or row state that N-1 reads.
6. New code reads the values N-1 writes and the columns it does not clear.

This check reads each up.sql (PostgreSQL and SQLite) from migration 072 on
and fails on a statement that the previous minor's server may not survive:

- DROP TABLE, DROP COLUMN, RENAME;
- SET NOT NULL;
- ADD COLUMN ... NOT NULL without DEFAULT;
- CREATE UNIQUE INDEX, ADD CONSTRAINT.

A file passes with such a statement when it has a line
`-- compat: <why the previous minor keeps working, or the changelog entry>`.

Rules 5 and 6 cannot be linted: the nightly previous-minor job (the last
release's tests on a schema the head migrated) and the upgrade test check
them.

Usage: check_migration_compat.py [--self-test] [FILE ...]
With no FILE, every linted migration of the repository.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MIGRATIONS = ROOT / "crates" / "cloacina" / "src" / "database" / "migrations"
# Migrations before this number are from before the rule.
FIRST_LINTED = 72

COMPAT_LINE = re.compile(r"^\s*--\s*compat:\s*\S", re.IGNORECASE | re.MULTILINE)

FORBIDDEN = [
    (re.compile(r"\bDROP\s+TABLE\b"), "DROP TABLE"),
    (re.compile(r"\bDROP\s+COLUMN\b"), "DROP COLUMN"),
    (re.compile(r"\bRENAME\b"), "RENAME"),
    (re.compile(r"\bSET\s+NOT\s+NULL\b"), "SET NOT NULL"),
    (re.compile(r"\bCREATE\s+UNIQUE\s+INDEX\b"), "CREATE UNIQUE INDEX"),
    (re.compile(r"\bADD\s+CONSTRAINT\b"), "ADD CONSTRAINT"),
]

ADD_COLUMN = re.compile(r"\bADD\s+(?:COLUMN\s+)?(?!CONSTRAINT\b)")


def statements(sql: str) -> list[str]:
    """The statements of `sql`, comments out, upper case, one space."""
    no_comments = re.sub(r"--[^\n]*", "", sql)
    no_comments = re.sub(r"/\*.*?\*/", "", no_comments, flags=re.DOTALL)
    out = []
    for raw in no_comments.split(";"):
        text = " ".join(raw.split()).upper()
        if text:
            out.append(text)
    return out


def findings(sql: str) -> list[str]:
    """What the lint refuses in `sql`, before the `-- compat:` override."""
    found = []
    for statement in statements(sql):
        for pattern, name in FORBIDDEN:
            if pattern.search(statement):
                found.append(f"{name}: {statement}")
        if statement.startswith("ALTER TABLE"):
            for column in ADD_COLUMN.split(statement)[1:]:
                if re.search(r"\bNOT\s+NULL\b", column) and not re.search(
                    r"\bDEFAULT\b", column
                ):
                    found.append(f"ADD COLUMN NOT NULL without DEFAULT: {statement}")
    return found


def check(sql: str) -> list[str]:
    if COMPAT_LINE.search(sql):
        return []
    return findings(sql)


def linted_files() -> list[Path]:
    files = []
    for backend in ("postgres", "sqlite"):
        for folder in sorted((MIGRATIONS / backend).iterdir()):
            match = re.match(r"^(\d+)_", folder.name)
            if not match or int(match.group(1)) < FIRST_LINTED:
                continue
            up = folder / "up.sql"
            if up.is_file():
                files.append(up)
    return files


def self_test() -> int:
    cases = [
        ("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT NOT NULL);", True),
        ("ALTER TABLE t ADD COLUMN c TEXT;", True),
        ("ALTER TABLE t ADD COLUMN c INTEGER NOT NULL DEFAULT 5;", True),
        ("CREATE INDEX IF NOT EXISTS i ON t (c);", True),
        ("-- DROP TABLE in a comment\nALTER TABLE t ADD COLUMN c TEXT;", True),
        ("DROP TABLE t;", False),
        ("ALTER TABLE t DROP COLUMN c;", False),
        ("ALTER TABLE t RENAME COLUMN a TO b;", False),
        ("ALTER TABLE t ALTER COLUMN c SET NOT NULL;", False),
        ("ALTER TABLE t ADD COLUMN c INTEGER NOT NULL;", False),
        ("ALTER TABLE t ADD c INTEGER NOT NULL;", False),
        (
            "ALTER TABLE t ADD COLUMN a INT NOT NULL DEFAULT 1, ADD COLUMN b INT NOT NULL;",
            False,
        ),
        ("CREATE UNIQUE INDEX u ON t (c);", False),
        ("ALTER TABLE t ADD CONSTRAINT k UNIQUE (c);", False),
        ("-- compat: the previous minor never reads t\nDROP TABLE t;", True),
    ]
    failed = 0
    for sql, passes in cases:
        got = not check(sql)
        if got != passes:
            failed += 1
            print(f"self-test: expected {'pass' if passes else 'fail'}: {sql!r}")
    if failed:
        print(f"self-test: {failed} case(s) wrong")
        return 1
    print(f"self-test: {len(cases)} cases right")
    return 0


def main(argv: list[str]) -> int:
    if argv and argv[0] == "--self-test":
        return self_test()
    files = [Path(a) for a in argv] if argv else linted_files()
    bad = 0
    for path in files:
        for finding in check(path.read_text()):
            bad += 1
            print(f"{path.relative_to(ROOT) if path.is_absolute() else path}: {finding}")
    if bad:
        print(
            f"\n{bad} statement(s) the previous minor's server may not survive. "
            "Move the removal one minor later, or add a line "
            "`-- compat: <why the previous minor keeps working>` to the file "
            "(CLOACINA-T-1087)."
        )
        return 1
    print(f"migration compat: {len(files)} up.sql file(s) checked")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
