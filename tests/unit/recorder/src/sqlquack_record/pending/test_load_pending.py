"""Loading pending case files."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.recorder.src.sqlquack_record.pending._test_types import LoadPendingTestCase
from tests.unit.recorder.src.sqlquack_record.pending.helpers import (
    CASE_HASH,
    PENDING_FILE,
    references,
)


@pytest.mark.parametrize(
    "test_case",
    (
        LoadPendingTestCase(
            description="a file as sqlquack pending writes it loads",
            file_name=f"{CASE_HASH}.json",
            replace="",
            with_text="",
            expected_references=("numeric/addition.toml#mixed_scales",),
        ),
        LoadPendingTestCase(
            description="files not named by a case hash are ignored",
            file_name="notes.json",
            replace="",
            with_text="",
            expected_references=(),
        ),
        LoadPendingTestCase(
            description="a file whose name and hash differ is rejected",
            file_name=f"{'0' * 64}.json",
            replace="",
            with_text="",
            expected_references=None,
        ),
        LoadPendingTestCase(
            description="an unknown key is rejected",
            file_name=f"{CASE_HASH}.json",
            replace='"format": 1,',
            with_text='"format": 1, "tolerance": 1,',
            expected_references=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pending_file_when_loading_then_only_exact_case_files_load(
    test_case: LoadPendingTestCase, tmp_path: Path
) -> None:
    loaded: tuple[str, ...] | None = references(
        directory=tmp_path,
        file_name=test_case.file_name,
        text=PENDING_FILE.replace(test_case.replace, test_case.with_text, 1),
    )

    assert loaded == test_case.expected_references
