"""Checking the blocks a website page marks as examples against what the commands print."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.scripts.website_examples._test_types import ExampleCheckTestCase
from tests.unit.scripts.website_examples.helpers import checked_site

TRANSCRIPT: str = (
    "{/* example: orders */}\n```console\n$ sqlquack convert query.sql\nSELECT 1\n"
    "sqlquack convert: done, converted\n```\n"
)
PRINTED: str = "SELECT 1\nsqlquack convert: done, converted\n"


@pytest.mark.parametrize(
    "test_case",
    (
        ExampleCheckTestCase(
            description="a transcript showing the real output passes",
            page=TRANSCRIPT,
            homepage="",
            printed=PRINTED,
            expected_return_code=0,
            expected_fragment="passed, 1 blocks",
        ),
        ExampleCheckTestCase(
            description="a leading progress line the page leaves out passes",
            page=TRANSCRIPT,
            homepage="",
            printed="sqlquack convert: converting…\n" + PRINTED,
            expected_return_code=0,
            expected_fragment="passed",
        ),
        ExampleCheckTestCase(
            description="a line of only an ellipsis stands for several lines",
            page="{/* example: orders */}\n```console\n$ sqlquack convert query.sql\n…\n"
            "sqlquack convert: done, converted\n```\n",
            homepage="",
            printed="SELECT\n    1\n" + "sqlquack convert: done, converted\n",
            expected_return_code=0,
            expected_fragment="passed",
        ),
        ExampleCheckTestCase(
            description="an ellipsis in a line stands for the rest of its text",
            page="{/* example: orders */}\n```console\n$ sqlquack convert query.sql\nSELECT 1\n"
            "sqlquack convert: done, …\n```\n",
            homepage="",
            printed=PRINTED,
            expected_return_code=0,
            expected_fragment="passed",
        ),
        ExampleCheckTestCase(
            description="a file shown under its label comment passes",
            page="{/* example: orders file=query.sql */}\n```sql\n-- query.sql\nSELECT a\nFROM t\n```\n",
            homepage="",
            printed="",
            expected_return_code=0,
            expected_fragment="passed",
        ),
        ExampleCheckTestCase(
            description="a homepage terminal reads its command spans and entities",
            page="",
            homepage='<pre class="terminal" data-example="orders">'
            '<span class="cmd">sqlquack convert query.sql</span>\n'
            "SELECT &quot;A&quot;</pre>",
            printed='SELECT "A"\n',
            expected_return_code=0,
            expected_fragment="passed",
        ),
        ExampleCheckTestCase(
            description="a value a Python block shows in its result comment passes",
            page="{/* example: no-inputs python */}\n```python\ntotal = 1 + 1\ntotal  # 2\n```\n",
            homepage="",
            printed="",
            expected_return_code=0,
            expected_fragment="passed",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pages_showing_real_output_when_checking_then_the_check_passes(
    test_case: ExampleCheckTestCase, tmp_path: Path
) -> None:
    code, messages = checked_site(
        tmp_path=tmp_path, pages=(test_case.page, test_case.homepage), printed=test_case.printed
    )
    assert code == test_case.expected_return_code, messages
    assert test_case.expected_fragment in messages


@pytest.mark.parametrize(
    "test_case",
    (
        ExampleCheckTestCase(
            description="output that changed fails with the difference",
            page=TRANSCRIPT,
            homepage="",
            printed="SELECT 2\nsqlquack convert: done, converted\n",
            expected_return_code=1,
            expected_fragment="+SELECT 2",
        ),
        ExampleCheckTestCase(
            description="a console block that names no example fails",
            page="```console\n$ sqlquack convert query.sql\nSELECT 1\n```\n",
            homepage="",
            printed=PRINTED,
            expected_return_code=1,
            expected_fragment="page.mdx:1: the block names no example",
        ),
        ExampleCheckTestCase(
            description="a homepage terminal that names no example fails",
            page="",
            homepage='<pre class="terminal"><span class="cmd">sqlquack check</span></pre>',
            printed="",
            expected_return_code=1,
            expected_fragment="index.html:1: the block names no example",
        ),
        ExampleCheckTestCase(
            description="an example without inputs fails",
            page="{/* example: missing */}\n```console\n$ sqlquack convert query.sql\n```\n",
            homepage="",
            printed="",
            expected_return_code=1,
            expected_fragment="website/examples/missing does not exist",
        ),
        ExampleCheckTestCase(
            description="a shown file that differs from the example's fails",
            page="{/* example: orders file=query.sql */}\n```sql\nSELECT b\nFROM t\n```\n",
            homepage="",
            printed="",
            expected_return_code=1,
            expected_fragment="query.sql differs from the page",
        ),
        ExampleCheckTestCase(
            description="a Python result comment that differs fails",
            page="{/* example: no-inputs python */}\n```python\n1 + 1  # 3\n```\n",
            homepage="",
            printed="",
            expected_return_code=1,
            expected_fragment="line 1 shows 3, but gives 2",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pages_that_differ_from_real_output_when_checking_then_the_check_fails(
    test_case: ExampleCheckTestCase, tmp_path: Path
) -> None:
    code, messages = checked_site(
        tmp_path=tmp_path, pages=(test_case.page, test_case.homepage), printed=test_case.printed
    )
    assert code == test_case.expected_return_code, messages
    assert test_case.expected_fragment in messages
