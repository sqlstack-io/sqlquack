"""An interaction's pair, named by the targets of two options or alternatives of its form."""

from __future__ import annotations

import pytest

from scripts.grammar_obligations._helpers.obligations import obligation_rows
from scripts.grammar_obligations.exceptions import ObligationError
from scripts.grammar_obligations.types import Row
from tests.unit.scripts.grammar_obligations._test_types import (
    InteractionPairTestCase,
    RefusedPairTestCase,
)
from tests.unit.scripts.grammar_obligations.helpers import (
    CHOICE,
    FORM,
    OPTION,
    SHARED,
    numbered_with,
)


@pytest.mark.parametrize(
    "test_case",
    (
        InteractionPairTestCase(
            description="an option and an alternative",
            pair=(OPTION, CHOICE),
            expected_pair=[f"{FORM}.o001", f"{FORM}.o002"],
        ),
        InteractionPairTestCase(
            description="a target both an option and an alternative name is the option",
            pair=(CHOICE, SHARED),
            expected_pair=[f"{FORM}.o002", f"{FORM}.o004"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_interaction_when_its_rows_are_written_then_its_pair_names_two_obligations(
    test_case: InteractionPairTestCase,
) -> None:
    rows: list[Row] = obligation_rows(numbered_with(test_case.pair))

    assert rows[-1]["pair"] == test_case.expected_pair


@pytest.mark.parametrize(
    "test_case",
    (
        RefusedPairTestCase(
            description="a target only the prose names",
            pair=(OPTION, "OVER"),
            expected_message="is not two options or alternatives",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_interaction_naming_no_option_when_its_rows_are_written_then_it_is_refused(
    test_case: RefusedPairTestCase,
) -> None:
    with pytest.raises(ObligationError, match=test_case.expected_message):
        obligation_rows(numbered_with(test_case.pair))
