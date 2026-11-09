"""How shared code is kept from comparing a dialect's name instead of asking its profile."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from scripts.fensu_policy._helpers.dialect_features import dialect_name_lines
from scripts.fensu_policy.constants import DEFAULT_DIALECTS
from scripts.fensu_policy.models import SourceLine
from tests.unit.scripts.fensu_policy._test_types import (
    DialectNameLinesTestCase,
    DialectNameRuleTestCase,
)

REPOSITORY: Path = Path(__file__).resolve().parents[4]
WORKSPACE_MANIFEST: str = '[workspace]\nmembers = ["crates/sqlquack-semantics"]\nresolver = "2"\n'
CRATE_MANIFEST: str = (
    '[package]\nname = "sqlquack-semantics"\nversion = "0.1.0"\nedition = "2021"\n\n'
    "[features]\ndefault = []\nsnowflake = []\n"
)
LIB: str = '#[cfg(feature = "snowflake")]\nmod snowflake;\nmod lineage;\n'
NAME_RULE_CONFIG: str = (
    '[targets.rust]\nanalyzer = "rust"\nroots = ["crates"]\ntests = []\n'
    'rule_modules = ["scripts.fensu_policy.rules.dialect_names"]\nselect = ["XSQ008"]\n'
)


@pytest.mark.parametrize(
    "test_case",
    [
        DialectNameLinesTestCase(
            "comparing a profile's name fails",
            "fn folds(context: Context) -> bool {\n"
            "    context.profile.name() == SNOWFLAKE_PROFILE\n}\n",
            (2,),
        ),
        DialectNameLinesTestCase(
            "matching on a profile's name fails, wherever the call sits",
            "match level.context.profile.name() {\n    _ => 0,\n}\nlet name = profile().name();\n",
            (1, 4),
        ),
        DialectNameLinesTestCase(
            "a dialect's profile-name constant fails on its own",
            'pub(crate) const SQLSERVER_PROFILE: &str = "SQL Server";\n'
            "let engine = [DUCKDB_PROFILE, POSTGRES_PROFILE];\n",
            (1, 2),
        ),
        DialectNameLinesTestCase(
            "a receiver whose name ends in profile fails, called or not",
            "let left = source_profile.name();\nlet right = target_profile().name();\n",
            (1, 2),
        ),
        DialectNameLinesTestCase(
            "a chain the formatter splits fails on the line the profile starts",
            "let engine = level\n    .context\n    .profile\n    .name()\n    == DUCKDB;\n"
            "let other = target_profile()\n    .name();\n",
            (3, 6),
        ),
        DialectNameLinesTestCase(
            "the trait's name called as a path fails",
            "let name = DialectProfile::name(profile);\nlet split = DialectProfile::\n    name(p);\n",
            (1, 2),
        ),
        DialectNameLinesTestCase(
            "asking the profile a question passes",
            "if context.profile.folds_unquoted_to_upper() {\n"
            "    context.profile.resolved_identifier(name, false)\n}\n",
            (),
        ),
        DialectNameLinesTestCase(
            "a profile constant no dialect names passes",
            'pub const DEFAULT_SESSION_PROFILE: &str = "utc-v1";\n'
            'const SHOWPLAN: &str = "SHOWPLAN_XML_STATISTICS_PROFILE";\n',
            (),
        ),
        DialectNameLinesTestCase(
            "a name read in a string or comment passes, and code after the string does not",
            '// context.profile.name() == SNOWFLAKE_PROFILE\nlet s = "profile.name()";\n'
            'let t = "/*"; let u = SNOWFLAKE_PROFILE;\n',
            (3,),
        ),
        DialectNameLinesTestCase(
            "a name method of anything but a profile passes",
            "let kind = family.name();\nlet word = profile.named();\n",
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_source_when_checking_then_lists_dialect_name_lines(
    test_case: DialectNameLinesTestCase,
) -> None:
    found: tuple[SourceLine, ...] = dialect_name_lines(
        source=test_case.source, dialects=DEFAULT_DIALECTS
    )

    assert tuple(line.line for line in found) == test_case.expected_lines


@pytest.mark.parametrize(
    "test_case",
    [
        DialectNameRuleTestCase(
            "a comparison in shared code is reported",
            "src/lineage/mod.rs",
            "pub fn folds(p: &dyn P) -> bool {\n    p.profile.name() == SNOWFLAKE_PROFILE\n}\n",
            1,
        ),
        DialectNameRuleTestCase(
            "the same comparison inside the dialect's folder is not",
            "src/snowflake/mod.rs",
            "pub fn folds(p: &dyn P) -> bool {\n    p.profile.name() == SNOWFLAKE_PROFILE\n}\n",
            0,
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_crate_when_running_fensu_then_reports_shared_name_comparisons(
    test_case: DialectNameRuleTestCase, tmp_path: Path
) -> None:
    crate: Path = tmp_path / "crates" / "sqlquack-semantics"
    for folder in ("snowflake", "lineage"):
        (crate / "src" / folder).mkdir(parents=True)
        (crate / "src" / folder / "mod.rs").write_text("pub fn profile() {}\n")
    (tmp_path / "Cargo.toml").write_text(WORKSPACE_MANIFEST)
    (crate / "Cargo.toml").write_text(CRATE_MANIFEST)
    (crate / "src" / "lib.rs").write_text(LIB)
    (crate / test_case.path).write_text(test_case.source)
    (tmp_path / "fensu.toml").write_text(NAME_RULE_CONFIG)
    shutil.copytree(
        REPOSITORY / "scripts" / "fensu_policy",
        tmp_path / "scripts" / "fensu_policy",
        ignore=shutil.ignore_patterns("__pycache__"),
    )
    (tmp_path / "scripts" / "__init__.py").write_text("")

    completed: subprocess.CompletedProcess[str] = subprocess.run(
        [str(Path(sys.executable).with_name("fensu")), "check", "--no-cache"],
        cwd=tmp_path,
        env={**os.environ, "PYTHONPATH": str(tmp_path)},
        capture_output=True,
        text=True,
        check=False,
    )

    output: str = completed.stdout + completed.stderr
    assert completed.returncode == min(test_case.expected_fault_count, 1), output
    assert output.count("XSQ008") == test_case.expected_fault_count, output
