"""The workspace lockfile guard against a real cargo workspace."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.workspace_locks.main.check import run_workspace_locks
from tests.unit.scripts.workspace_locks._test_types import LockTestCase
from tests.unit.scripts.workspace_locks.helpers import (
    ORDERS_MANIFEST,
    ORDERS_WITH_CUSTOMERS,
    write_locked_workspace,
)


@pytest.mark.parametrize(
    "test_case",
    (
        LockTestCase(
            description="a lockfile that holds every dependency passes",
            manifest_after_lock=ORDERS_MANIFEST,
            expected_return_code=0,
            expected_fragment="passed, 1 workspaces resolved with --locked",
        ),
        LockTestCase(
            description="a dependency added after locking fails",
            manifest_after_lock=ORDERS_WITH_CUSTOMERS,
            expected_return_code=1,
            expected_fragment="workspace locks: workspace: error: cannot update the lock file",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_workspace_when_resolving_locked_then_fails_only_on_a_stale_lockfile(
    test_case: LockTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    write_locked_workspace(directory=tmp_path, manifest_after_lock=test_case.manifest_after_lock)

    return_code: int = run_workspace_locks(
        ["--repository", str(tmp_path), "--workspace", "workspace"]
    )

    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in capsys.readouterr().err
