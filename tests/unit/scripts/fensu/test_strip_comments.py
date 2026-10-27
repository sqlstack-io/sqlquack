"""Removing non-doc comments from Rust source."""

from __future__ import annotations

import pytest

from scripts.fensu._helpers.rust.comments import strip_comments
from tests.unit.scripts.fensu._test_types import TransformTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        TransformTestCase(
            description="a comment alone on its line goes with its line",
            source="fn f() {\n    // explain\n    g();\n}\n",
            expected_source="fn f() {\n    g();\n}\n",
        ),
        TransformTestCase(
            description="a trailing comment goes with the blanks before it",
            source="let x = 1; // one\n",
            expected_source="let x = 1;\n",
        ),
        TransformTestCase(
            description="an inline block comment goes with the blanks after it",
            source="f(/* a */ b);\n",
            expected_source="f(b);\n",
        ),
        TransformTestCase(
            description="nested block comments end at their own close",
            source="/* a /* b */ c */\nfn f() {}\n",
            expected_source="fn f() {}\n",
        ),
        TransformTestCase(
            description="doc comments stay",
            source="//! Module.\n/// Item.\n/** Block. */\nfn f() {}\n",
            expected_source="//! Module.\n/// Item.\n/** Block. */\nfn f() {}\n",
        ),
        TransformTestCase(
            description="four slashes are not a doc comment",
            source="//// rule\nfn f() {}\n",
            expected_source="fn f() {}\n",
        ),
        TransformTestCase(
            description="comment markers inside strings and raw strings stay",
            source='let a = "// x"; let b = r#"/* "y" */"#; let c = \'/\';\n',
            expected_source='let a = "// x"; let b = r#"/* "y" */"#; let c = \'/\';\n',
        ),
        TransformTestCase(
            description="a lifetime is not a character literal",
            source="fn f<'a>(x: &'a str) -> &'a str { x } // done\n",
            expected_source="fn f<'a>(x: &'a str) -> &'a str { x }\n",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_rust_source_when_stripping_comments_then_only_doc_comments_remain(
    test_case: TransformTestCase,
) -> None:
    assert strip_comments(text=test_case.source) == test_case.expected_source, test_case.description
