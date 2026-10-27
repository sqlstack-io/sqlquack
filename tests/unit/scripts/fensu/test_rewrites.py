"""Single-rule rewrites of Rust source: Debug derives, unreachable! and unwraps."""

from __future__ import annotations

import pytest

from tests.unit.scripts.fensu._test_types import TransformTestCase
from tests.unit.scripts.fensu.helpers import (
    derived,
    named_literals,
    without_unreachable,
    without_unwraps,
)


@pytest.mark.parametrize(
    "test_case",
    (
        TransformTestCase(
            description="a model without derives gets one",
            source="/// Order.\npub struct Order;\n",
            expected_source="/// Order.\n#[derive(Debug)]\npub struct Order;\n",
        ),
        TransformTestCase(
            description="Debug joins an existing derive list",
            source="#[derive(Clone)]\nenum Status { Open }\n",
            expected_source="#[derive(Debug, Clone)]\nenum Status { Open }\n",
        ),
        TransformTestCase(
            description="a derived or hand-written Debug stays",
            source="#[derive(Debug)]\nstruct A;\nstruct B;\nimpl fmt::Debug for B {}\n",
            expected_source="#[derive(Debug)]\nstruct A;\nstruct B;\nimpl fmt::Debug for B {}\n",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_models_when_deriving_debug_then_each_model_derives_it_once(
    test_case: TransformTestCase,
) -> None:
    assert derived(test_case.source) == test_case.expected_source, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        TransformTestCase(
            description="a statement returns an internal error",
            source="fn f() -> Result<u8> {\n    if x { unreachable!() }\n    Ok(1)\n}\n",
            expected_source=(
                "fn f() -> Result<u8> {\n    if x { return Err(crate::tokens::errors::Error::"
                'internal("the parser reached a state its checks rule out")) }\n    Ok(1)\n}\n'
            ),
        ),
        TransformTestCase(
            description="a tail arm keeps its message and needs no return",
            source='fn f() -> Result<u8> {\n    match x { 1 => Ok(1), _ => unreachable!("one") }\n}\n',
            expected_source=(
                "fn f() -> Result<u8> {\n    match x { 1 => Ok(1), _ => "
                'Err(crate::tokens::errors::Error::internal("one")) }\n}\n'
            ),
        ),
        TransformTestCase(
            description="formatted messages become format!",
            source='fn f() -> Result<u8> {\n    unreachable!("{} left", n);\n}\n',
            expected_source=(
                "fn f() -> Result<u8> {\n    return Err(crate::tokens::errors::Error::"
                'internal(format!("{} left", n)));\n}\n'
            ),
        ),
        TransformTestCase(
            description="functions that do not return Result stay",
            source="fn f() -> u8 {\n    unreachable!()\n}\n",
            expected_source="fn f() -> u8 {\n    unreachable!()\n}\n",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_unreachable_when_rewriting_then_result_functions_return_errors(
    test_case: TransformTestCase,
) -> None:
    assert without_unreachable(test_case.source) == test_case.expected_source, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        TransformTestCase(
            description="unwrap propagates a generic internal error",
            source="fn f() -> Result<u8> {\n    Ok(x.pop().unwrap())\n}\n",
            expected_source=(
                "fn f() -> Result<u8> {\n    Ok(x.pop().ok_or_else(|| crate::tokens::errors::"
                'Error::internal("a value the parser checked is missing"))?)\n}\n'
            ),
        ),
        TransformTestCase(
            description="expect keeps its message",
            source='fn f() -> Result<u8> {\n    Ok(x.expect("set"))\n}\n',
            expected_source=(
                "fn f() -> Result<u8> {\n    Ok(x.ok_or_else(|| crate::tokens::errors::"
                'Error::internal("set"))?)\n}\n'
            ),
        ),
        TransformTestCase(
            description="functions that do not return Result stay",
            source="fn f() -> u8 {\n    x.unwrap()\n}\n",
            expected_source="fn f() -> u8 {\n    x.unwrap()\n}\n",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_unwrap_when_rewriting_then_result_functions_propagate_errors(
    test_case: TransformTestCase,
) -> None:
    assert without_unwraps(test_case.source) == test_case.expected_source, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        TransformTestCase(
            description="both sides of == and != are named",
            source='if kind == "PROCEDURE" || "VIEW" != kind {}\n',
            expected_source="if kind == PROCEDURE || VIEW != kind {}\n",
        ),
        TransformTestCase(
            description="lower-case and symbol literals take their override names",
            source='if text == "e" && sign == "%" {}\n',
            expected_source="if text == LOWERCASE_ESCAPE_STRING_PREFIX && sign == PERCENT_SIGN {}\n",
        ),
        TransformTestCase(
            description="literals outside comparisons stay",
            source='let kind = "VIEW"; f("TABLE"); if a <= "b" {}\n',
            expected_source='let kind = "VIEW"; f("TABLE"); if a <= "b" {}\n',
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_compared_literals_when_naming_then_each_becomes_its_constant(
    test_case: TransformTestCase,
) -> None:
    assert named_literals(test_case.source) == test_case.expected_source, test_case.description
