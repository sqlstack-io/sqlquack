"""A case that changes account- or catalog-level state, or cannot be placed, is held before a run."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.databricks_receipts._helpers.run import record
from scripts.databricks_receipts._helpers.scope import held_reason
from scripts.databricks_receipts.exceptions import RefereeError
from tests.unit.scripts.databricks_receipts._test_types import HeldRunTestCase, ScopeTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        ScopeTestCase(
            description="a catalog created",
            sql="CREATE CATALOG v1",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a governed tag created or replaced",
            sql="CREATE OR REPLACE GOVERNED TAG v1 VALUES ( v1 )",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a schema of the catalog created",
            sql="CREATE SCHEMA IF NOT EXISTS s1",
            expected_held=True,
        ),
        ScopeTestCase(
            description="an external location dropped",
            sql="DROP EXTERNAL LOCATION v1",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a storage credential renamed",
            sql="ALTER STORAGE CREDENTIAL v1 RENAME TO v2",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a comment on a catalog",
            sql="COMMENT ON CATALOG v1 IS 'x'",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a foreign catalog refreshed",
            sql="REFRESH FOREIGN CATALOG v1",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a tag set on a catalog",
            sql="SET TAG ON CATALOG v1 k",
            expected_held=True,
        ),
        ScopeTestCase(
            description="tags unset on a schema",
            sql="UNSET TAGS ON SCHEMA s1 ( k )",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a grant on any file",
            sql="GRANT SELECT ON ANY FILE TO `u1`",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a grant to a share recipient",
            sql="GRANT SELECT ON SHARE v1 TO RECIPIENT v2",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a revoke on the workspace catalog",
            sql="REVOKE USE CATALOG ON CATALOG workspace FROM `account users`",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a file put into a volume",
            sql="PUT 'm' INTO '/Volumes/workspace/s1/v1/m'",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a schema created inside a script block",
            sql="BEGIN IF 1 = 1 THEN CREATE SCHEMA s1 ; END IF ; END",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a catalog dropped by EXECUTE IMMEDIATE",
            sql="EXECUTE IMMEDIATE 'DROP CATALOG v1 CASCADE'",
            expected_held=True,
        ),
        ScopeTestCase(
            description="a statement word the referee cannot place",
            sql="FROB t1",
            expected_held=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_account_level_or_unplaceable_case_when_scoped_then_it_is_held(
    test_case: ScopeTestCase,
) -> None:
    assert (held_reason(test_case.sql) is not None) is test_case.expected_held


@pytest.mark.parametrize(
    "test_case",
    (
        ScopeTestCase(
            description="a query",
            sql="SELECT 1",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a view with schema binding",
            sql="CREATE VIEW v2 WITH SCHEMA BINDING AS SELECT 1",
            expected_held=False,
        ),
        ScopeTestCase(
            description="an external volume",
            sql="CREATE EXTERNAL VOLUME v1 LOCATION '/tmp/x'",
            expected_held=False,
        ),
        ScopeTestCase(
            description="tags set on a table by ALTER TABLE",
            sql="ALTER TABLE t1 SET TAGS ( 'k' = 'v' )",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a tag set on a table",
            sql="SET TAG ON TABLE t1 k",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a grant on a table",
            sql="GRANT SELECT ON TABLE t1 TO `account users`",
            expected_held=False,
        ),
        ScopeTestCase(
            description="grants to a recipient shown",
            sql="SHOW GRANTS TO RECIPIENT v1",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a catalog described",
            sql="DESCRIBE CATALOG v1",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a catalog used",
            sql="USE CATALOG v1",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a script beginning with a FOR label",
            sql="BEGIN :FOR SELECT 1 DO v1 ; END FOR ; END",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a labelled block",
            sql="l1 : BEGIN SELECT 1 ; END l1",
            expected_held=False,
        ),
        ScopeTestCase(
            description="a text beginning with a name",
            sql="t1 SET c1 = 1",
            expected_held=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_schema_case_when_scoped_then_it_runs(test_case: ScopeTestCase) -> None:
    assert (held_reason(test_case.sql) is not None) is test_case.expected_held


@pytest.mark.parametrize(
    "test_case",
    (
        HeldRunTestCase(
            description="a query and a catalog created",
            cases=(
                {"case_id": "db.x.k1", "sql": "SELECT 1"},
                {"case_id": "db.x.k2", "sql": "CREATE CATALOG v1"},
            ),
            expected_error="1 cases change account- or catalog-level state",
            expected_reports=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_held_case_when_recorded_then_nothing_runs_and_no_receipt_is_written(
    test_case: HeldRunTestCase, tmp_path: Path
) -> None:
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record(cases=list(test_case.cases), out=out, private=tmp_path / "private", seed=None)

    assert not out.exists()
    assert len(list((tmp_path / "private").glob("*.held.json"))) == test_case.expected_reports
