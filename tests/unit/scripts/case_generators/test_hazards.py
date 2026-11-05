"""The hazard generator: one case per argument family's hazards, isolated hazards apart."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.case_generators.models import Case
from tests.unit.scripts.case_generators._test_types import (
    ExpectationTestCase,
    FindingTestCase,
    HazardTestCase,
)
from tests.unit.scripts.case_generators.helpers import coverage_entries, hazard_cases


@pytest.mark.parametrize(
    "test_case",
    (
        HazardTestCase(
            "a family's hazards share one case, the other argument benign",
            "fn_get_1_text_hazards",
            (("PARSE_JSON('{}')", "''"), ("PARSE_JSON('{}')", "'$.a'")),
        ),
        HazardTestCase(
            "an isolated hazard gets a case of its own",
            "fn_get_0_variant_duplicate_keys",
            (('PARSE_JSON(\'{"a":1,"a":2}\')', "'a'"),),
        ),
        HazardTestCase(
            "a probe's extra rows make a case of their own",
            "fn_get_probe_0_extra_rows",
            (("PARSE_JSON('{\"k\":1}')", "'k'"),),
        ),
        HazardTestCase(
            "the family's other hazards leave out the isolated one",
            "fn_get_0_variant_hazards",
            (('PARSE_JSON(\'{"a":{"b":1}}\')', "'a'"),),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_declared_form_when_generating_hazards_then_each_argument_family_gets_its_hazards(
    tmp_path: Path, test_case: HazardTestCase
) -> None:
    cases: dict[str, Case] = hazard_cases(tmp_path)
    assert cases[test_case.case_id].rows == test_case.expected_rows, test_case.description
    assert cases[test_case.case_id].expression == "GET(x0, x1)", test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        FindingTestCase("GET of a computed key starting with $", "families.text", "dollar_prefix"),
        FindingTestCase("GET of a computed key starting with /", "families.text", "slash_prefix"),
        FindingTestCase("GET of the empty key and *", "families.text", "dots_brackets"),
        FindingTestCase("GET of keys a path reads specially", "families.variant", "path_keys"),
        FindingTestCase("ARRAY_POSITION of 1 among 1.0", "families.variant", "number_spellings"),
        FindingTestCase("FLOAT one written three ways", "families.float", "one_spellings"),
        FindingTestCase("INITCAP of title-case digraphs", "families.text", "title_case_digraph"),
        FindingTestCase("ANY_VALUE of a mixed group", "families.group", "mixed"),
        FindingTestCase("ANY_VALUE of NULL beside a value", "families.group", "null_beside_value"),
        FindingTestCase(
            "TIMESTAMP_NTZ(9) text at half a millisecond",
            "families.timestamp_ntz",
            "half_millisecond",
        ),
        FindingTestCase("window aggregates over explicit frames", "facets", "frame"),
        FindingTestCase("FLOAT to NUMBER beyond the recorded scales", "facets", "float_scale"),
        FindingTestCase("NOT ILIKE ANY written infix", "facets", "negated"),
    ),
    ids=lambda case: case.description,
)
def test_given_a_review_finding_when_reading_the_coverage_files_then_it_is_a_hazard_or_a_facet(
    test_case: FindingTestCase,
) -> None:
    assert test_case.expected_entry in coverage_entries()[test_case.table], test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        ExpectationTestCase(
            "a hazard no converted input holds expects its catalogue divergence alone",
            "fn_get_0_variant_long_number",
            "json_fidelity",
        ),
        ExpectationTestCase(
            "a form's divergence for one position, family and hazard isolates and expects it",
            "fn_get_1_text_slash_prefix",
            "json_fidelity",
        ),
        ExpectationTestCase(
            "the other hazards of the family share a case expecting nothing",
            "fn_get_1_text_hazards",
            None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_declared_divergences_when_generating_hazards_then_cases_expect_them(
    tmp_path: Path, test_case: ExpectationTestCase
) -> None:
    cases: dict[str, Case] = hazard_cases(tmp_path)
    assert cases[test_case.case_id].expected_divergence == test_case.expected_divergence, (
        test_case.description
    )


@pytest.mark.parametrize(
    "test_case",
    (
        HazardTestCase(
            "an unrecordable hazard gets no case and joins no family case",
            "fn_get_0_variant_hazards",
            (('PARSE_JSON(\'{"a":{"b":1}}\')', "'a'"),),
        ),
        HazardTestCase(
            "ordering keys hold each row's number and each row has its own group",
            "node_within_group_0_text_hazards",
            (("'g001'", "''", "1"), ("'g002'", "'$.a'", "2"), ("'g003'", "'/a'", "3")),
        ),
        HazardTestCase(
            "two arguments of one family differ in each order, then hold one equal row",
            "fn_contains_combination_0_text_text",
            (("'a'", "'b'"), ("'b'", "'ab'"), ("'ab'", "'a'"), ("'a'", "'a'")),
        ),
        HazardTestCase(
            "a hazard's position uses up no value, so the others of its family differ",
            "fn_contains_0_text_hazards",
            (("''", "'a'"), ("'$.a'", "'a'"), ("'/a'", "'a'")),
        ),
        HazardTestCase(
            "three arguments of a two-valued family each differ in a row of their own",
            "fn_decode_combination_0_boolean_boolean_boolean",
            (
                ("FALSE", "TRUE", "TRUE"),
                ("TRUE", "FALSE", "TRUE"),
                ("TRUE", "TRUE", "FALSE"),
                ("TRUE", "TRUE", "TRUE"),
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_row_declarations_when_generating_hazards_then_rows_follow_them(
    tmp_path: Path, test_case: HazardTestCase
) -> None:
    cases: dict[str, Case] = hazard_cases(tmp_path)
    assert "fn_get_0_variant_unrecorded" not in cases, test_case.description
    assert cases[test_case.case_id].rows == test_case.expected_rows, test_case.description
