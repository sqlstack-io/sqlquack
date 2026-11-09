"""Obligation targets shortened by the derivation."""

from __future__ import annotations

import re

import pytest

from scripts.grammar_obligations._helpers.derive import derive
from scripts.grammar_obligations.models import Draft
from tests.unit.scripts.grammar_obligations._test_types import ShortenedTargetTestCase

FRAME: str = (
    "{ RANGE UNBOUNDED PRECEDING | RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW"
    " | RANGE BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING }"
)


@pytest.mark.parametrize(
    "test_case",
    (
        ShortenedTargetTestCase(
            description="a window frame list longer than the target width",
            grammar=f"F ( ) OVER ( ORDER BY <expr> [ {FRAME} ] )",
            expected_last_tokens=frozenset(FRAME.split()),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_long_alternative_when_derived_then_its_targets_are_cut_after_a_whole_token(
    test_case: ShortenedTargetTestCase,
) -> None:
    drafts: list[Draft] = derive(form="sf.functions.f", grammar=[test_case.grammar])
    last: list[str] = re.findall(r"(\S+)…", "\n".join(draft.target for draft in drafts))

    assert last
    assert set(last) <= test_case.expected_last_tokens
