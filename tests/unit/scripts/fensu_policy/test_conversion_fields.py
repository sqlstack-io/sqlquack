"""How shared code is kept from reading the conversion's record of how the target holds values."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

from scripts.fensu_policy._helpers.dialect_features import conversion_field_lines
from scripts.fensu_policy.models import SourceLine
from tests.unit.scripts.fensu_policy._test_types import (
    ConversionFieldLinesTestCase,
    ConversionFieldRuleTestCase,
    MixedCheckTestCase,
)
from tests.unit.scripts.fensu_policy.helpers import run_fensu_check, write_semantics_crate

FIELD_RULE_CONFIG: str = (
    '[targets.rust]\nanalyzer = "rust"\nroots = ["crates"]\ntests = []\n'
    'rule_modules = ["scripts.fensu_policy.rules.conversion_fields"]\nselect = ["XSQ009"]\n'
)
# A Python target with a custom rule beside the Rust one, as the repository's own fensu.toml has.
MIXED_CONFIG: str = (
    '[targets.python]\nanalyzer = "python"\nroots = ["tooling"]\ntests = []\n'
    'rule_modules = ["scripts.fensu_policy.rules.script_names"]\nselect = ["XSQ005"]\n\n'
    + FIELD_RULE_CONFIG
)


@pytest.mark.parametrize(
    "test_case",
    [
        ConversionFieldLinesTestCase(
            "reading a holding field fails",
            "fn json(column: &Column) -> bool {\n    column.holding == Holding::Json\n}\n",
            (2,),
        ),
        ConversionFieldLinesTestCase(
            "every holding field fails, each on its line",
            "let a = c.typed_leaves || c.decimal_leaves;\nlet b = c.untainted;\n"
            "let d = c.sourced && c.utc_offset;\nlet e = &c.declared_offsets;\n"
            "let f = c.widened || c.suffixed;\n",
            (1, 2, 3, 4, 5),
        ),
        ConversionFieldLinesTestCase(
            "a read the formatter splits fails on the line the field is on",
            "let typed = column\n    .typed_leaves;\n",
            (2,),
        ),
        ConversionFieldLinesTestCase(
            "setting the fields in a struct literal passes",
            "ColumnSpec {\n    holding: Holding::Json,\n    widened: false,\n    ..column\n}\n",
            (),
        ),
        ConversionFieldLinesTestCase(
            "a method or a longer field of the same start passes",
            "let x = plan.holding();\nlet y = row.sourced_at;\nlet z = Holding::is_json;\n",
            (),
        ),
        ConversionFieldLinesTestCase(
            "destructuring a field from a column fails",
            "let ColumnSpec { holding, .. } = spec;\n",
            (1,),
        ),
        ConversionFieldLinesTestCase(
            "a match arm binding fields fails on each bound field's line",
            "match spec {\n    ColumnSpec {\n        name,\n        utc_offset: true,\n"
            "        ref mut declared_offsets,\n        ..\n    } => name,\n}\n",
            (4, 5),
        ),
        ConversionFieldLinesTestCase(
            "an if-let pattern without a rest, and a parameter pattern, fail",
            "if let ColumnSpec { widened: w, name, data_type } = spec {}\n"
            "fn f(ColumnSpec::<T> { sourced, .. }: ColumnSpec<T>) {}\n",
            (1, 2),
        ),
        ConversionFieldLinesTestCase(
            "a pattern binding only other fields, or a nested pattern's field, passes",
            "let ColumnSpec { name, data_type: Inner { holding, .. }, .. } = spec;\n",
            (),
        ),
        ConversionFieldLinesTestCase(
            "a literal building a column from a base passes, as do type headers",
            "let c = ColumnSpec {\n    holding,\n    suffixed: false,\n    ..base\n};\n"
            "impl ColumnSpec {\n    fn f() {}\n}\npub struct ColumnSpec<T> {\n    holding: H,\n}\n",
            (),
        ),
        ConversionFieldLinesTestCase(
            "a field named in a string or comment passes",
            '// column.holding\nlet s = "column.widened";\n',
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_source_when_checking_then_lists_conversion_field_reads(
    test_case: ConversionFieldLinesTestCase,
) -> None:
    found: tuple[SourceLine, ...] = conversion_field_lines(source=test_case.source)

    assert tuple(line.line for line in found) == test_case.expected_lines


@pytest.mark.parametrize(
    "test_case",
    [
        ConversionFieldRuleTestCase(
            "a read in shared code is reported",
            "src/schemas/mod.rs",
            "pub fn json(c: &C) -> bool {\n    c.holding == H::Json\n}\n",
            1,
        ),
        ConversionFieldRuleTestCase(
            "the same read inside the dialect's folder is not",
            "src/snowflake/mod.rs",
            "pub fn json(c: &C) -> bool {\n    c.holding == H::Json\n}\n",
            0,
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_crate_when_running_fensu_then_reports_shared_conversion_field_reads(
    test_case: ConversionFieldRuleTestCase, tmp_path: Path
) -> None:
    write_semantics_crate(tmp_path, FIELD_RULE_CONFIG)
    (tmp_path / "crates" / "sqlquack-semantics" / test_case.path).write_text(test_case.source)

    completed: subprocess.CompletedProcess[str] = run_fensu_check(tmp_path, os.environ["PATH"])

    output: str = completed.stdout + completed.stderr
    assert completed.returncode == min(test_case.expected_fault_count, 1), output
    assert output.count("XSQ009") == test_case.expected_fault_count, output


@pytest.mark.parametrize(
    "test_case",
    [
        MixedCheckTestCase(
            "with cargo, the check is complete",
            os.environ["PATH"],
            0,
            "Found 0 faults",
            "Check incomplete",
        ),
        MixedCheckTestCase(
            "without cargo, the check says it is incomplete",
            str(Path(sys.executable).parent),
            2,
            "Check incomplete",
            "Found ",
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_python_and_rust_targets_when_running_fensu_then_summary_says_whether_all_ran(
    test_case: MixedCheckTestCase, tmp_path: Path
) -> None:
    write_semantics_crate(tmp_path, MIXED_CONFIG)
    (tmp_path / "tooling").mkdir()
    (tmp_path / "tooling" / "__init__.py").write_text("")

    completed: subprocess.CompletedProcess[str] = run_fensu_check(tmp_path, test_case.path)

    output: str = completed.stdout + completed.stderr
    assert completed.returncode == test_case.expected_returncode, output
    assert test_case.expected_summary in output, output
    assert test_case.absent_summary not in output, output
