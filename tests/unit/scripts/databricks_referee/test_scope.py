"""A case that changes account- or catalog-level state, or cannot be placed, is held before a run."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.databricks_referee._helpers.run import record
from scripts.databricks_referee._helpers.scope import held_reason
from scripts.databricks_referee.exceptions import RefereeError


@pytest.mark.parametrize(
    "sql",
    [
        "CREATE CATALOG v1",
        "CREATE OR REPLACE GOVERNED TAG v1 VALUES ( v1 )",
        "CREATE SCHEMA IF NOT EXISTS s1",
        "DROP EXTERNAL LOCATION v1",
        "ALTER STORAGE CREDENTIAL v1 RENAME TO v2",
        "COMMENT ON CATALOG v1 IS 'x'",
        "REFRESH FOREIGN CATALOG v1",
        "SET TAG ON CATALOG v1 k",
        "UNSET TAGS ON SCHEMA s1 ( k )",
        "GRANT SELECT ON ANY FILE TO `u1`",
        "GRANT SELECT ON SHARE v1 TO RECIPIENT v2",
        "REVOKE USE CATALOG ON CATALOG workspace FROM `account users`",
        "PUT 'm' INTO '/Volumes/workspace/s1/v1/m'",
        "BEGIN IF 1 = 1 THEN CREATE SCHEMA s1 ; END IF ; END",
        "EXECUTE IMMEDIATE 'DROP CATALOG v1 CASCADE'",
        "FROB t1",
    ],
)
def test_given_an_account_level_or_unplaceable_case_when_scoped_then_it_is_held(sql: str) -> None:
    assert held_reason(sql) is not None


@pytest.mark.parametrize(
    "sql",
    [
        "SELECT 1",
        "CREATE VIEW v2 WITH SCHEMA BINDING AS SELECT 1",
        "CREATE EXTERNAL VOLUME v1 LOCATION '/tmp/x'",
        "ALTER TABLE t1 SET TAGS ( 'k' = 'v' )",
        "SET TAG ON TABLE t1 k",
        "GRANT SELECT ON TABLE t1 TO `account users`",
        "SHOW GRANTS TO RECIPIENT v1",
        "DESCRIBE CATALOG v1",
        "USE CATALOG v1",
        "BEGIN :FOR SELECT 1 DO v1 ; END FOR ; END",
        "l1 : BEGIN SELECT 1 ; END l1",
        "t1 SET c1 = 1",
    ],
)
def test_given_a_case_schema_case_when_scoped_then_it_runs(sql: str) -> None:
    assert held_reason(sql) is None


def test_given_a_held_case_when_recorded_then_nothing_runs_and_no_receipt_is_written(
    tmp_path: Path,
) -> None:
    cases = [
        {"case_id": "db.x.k1", "sql": "SELECT 1"},
        {"case_id": "db.x.k2", "sql": "CREATE CATALOG v1"},
    ]
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match="1 cases change account- or catalog-level state"):
        record(cases=cases, out=out, private=tmp_path / "private", seed=None)

    assert not out.exists()
    assert len(list((tmp_path / "private").glob("*.held.json"))) == 1
