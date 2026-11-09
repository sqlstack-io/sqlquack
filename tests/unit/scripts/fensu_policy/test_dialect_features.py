"""How a multi-dialect role crate gates each dialect's folder, dependencies and code."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from scripts.fensu_policy._helpers.dialect_features import (
    default_dialects,
    feature_gated_dependencies,
    folders_without_features,
    path_dialect,
    read_crate_features,
    shared_dialect_lines,
)
from scripts.fensu_policy._helpers.layout_names import variant_map
from scripts.fensu_policy.constants import DEFAULT_DIALECT_VARIANTS, DEFAULT_DIALECTS
from scripts.fensu_policy.models import CrateFeatures, DialectVocabulary, SourceLine
from tests.unit.scripts.fensu_policy._test_types import (
    FeatureGateTestCase,
    FolderFeatureTestCase,
    ManifestFaultTestCase,
    PathDialectTestCase,
    SharedLinesTestCase,
)

VOCABULARY: DialectVocabulary = DialectVocabulary(
    dialects=DEFAULT_DIALECTS,
    variants={
        variant: dialect
        for dialect, variant in variant_map(entries=DEFAULT_DIALECT_VARIANTS).items()
    },
    folders=frozenset({"snowflake", "clickhouse"}),
    crates={"sqlquack_syntax_duckdb": "duckdb", "sqlquack_types_snowflake": "snowflake"},
)

REPOSITORY: Path = Path(__file__).resolve().parents[4]
WORKSPACE_MANIFEST: str = '[workspace]\nmembers = ["crates/sqlquack-semantics"]\nresolver = "2"\n'
CRATE_MANIFEST: str = (
    '[package]\nname = "sqlquack-semantics"\nversion = "0.1.0"\nedition = "2021"\n\n'
)
GATED_MODULE: str = '#[cfg(feature = "snowflake")]\nmod snowflake;\n'
FOLDER_RULE_CONFIG: str = (
    '[targets.rust]\nanalyzer = "rust"\nroots = ["crates"]\ntests = []\n'
    'rule_modules = ["scripts.fensu_policy.rules.dialect_folders"]\nselect = ["XSQ007"]\n'
)

REGISTRY: str = (
    "match dialect {\n"
    '    #[cfg(feature = "snowflake")]\n'
    "    Dialect::Snowflake => Some(&snowflake::PROFILE),\n"
    '    #[cfg(feature = "clickhouse")] Dialect::ClickHouse => Some(&clickhouse::PROFILE),\n'
    "    _ => None,\n"
    "}\n"
)


@pytest.mark.parametrize(
    "test_case",
    [
        FeatureGateTestCase(
            "dependencies only their own dialect feature turns on are gated",
            "[features]\ndefault = []\n"
            'snowflake = ["dep:sqlquack-types-snowflake"]\n'
            'clickhouse = ["sqlquack-syntax-clickhouse/stacker"]\n'
            'extras = ["sqlquack-types-snowflake?/serde"]\n'
            "[dependencies]\nsqlquack-syntax.workspace = true\n"
            "sqlquack-types-snowflake = { workspace = true, optional = true }\n"
            "sqlquack-syntax-clickhouse = { workspace = true, optional = true }\n",
            frozenset({"sqlquack-types-snowflake", "sqlquack-syntax-clickhouse"}),
            (),
        ),
        FeatureGateTestCase(
            "a required dependency is not gated, even with a feature of its name",
            '[features]\nduckdb = []\n[dependencies]\nsqlquack-syntax-duckdb = "0.2"\n',
            frozenset(),
            (),
        ),
        FeatureGateTestCase(
            "another dialect's feature does not gate a dependency",
            '[features]\nsnowflake = ["dep:sqlquack-syntax-duckdb"]\n'
            "[dependencies]\nsqlquack-syntax-duckdb = { workspace = true, optional = true }\n",
            frozenset(),
            (),
        ),
        FeatureGateTestCase(
            "a dependency a non-dialect feature also turns on is not gated",
            '[features]\nsnowflake = ["dep:sqlquack-types-snowflake"]\n'
            'full = ["dep:sqlquack-types-snowflake"]\n'
            "[dependencies]\nsqlquack-types-snowflake = { workspace = true, optional = true }\n",
            frozenset(),
            (),
        ),
        FeatureGateTestCase(
            "default features that reach a dialect through another feature are reported",
            '[features]\ndefault = ["full"]\nfull = ["snowflake"]\n'
            'snowflake = ["dep:sqlquack-types-snowflake"]\n'
            "[dependencies]\nsqlquack-types-snowflake = { workspace = true, optional = true }\n",
            frozenset(),
            ("snowflake",),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_manifest_when_reading_features_then_finds_gates(
    test_case: FeatureGateTestCase,
) -> None:
    features: CrateFeatures = read_crate_features(manifest_text=test_case.manifest)

    assert feature_gated_dependencies(features=features, dialects=DEFAULT_DIALECTS) == (
        test_case.expected_gated
    )
    assert default_dialects(features=features, dialects=DEFAULT_DIALECTS) == (
        test_case.expected_defaults
    )


@pytest.mark.parametrize(
    "test_case",
    [
        PathDialectTestCase(
            "a file in a dialect folder", ("src", "snowflake", "mod.rs"), "snowflake"
        ),
        PathDialectTestCase("a dialect module file", ("src", "clickhouse.rs"), "clickhouse"),
        PathDialectTestCase("a shared module", ("src", "names", "mod.rs"), None),
        PathDialectTestCase("an integration test is not a folder", ("tests", "duckdb.rs"), None),
        PathDialectTestCase(
            "a dialect folder below a shared module is not a dialect folder",
            ("src", "dialects", "snowflake", "mod.rs"),
            None,
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_path_when_locating_then_returns_dialect_folder(
    test_case: PathDialectTestCase,
) -> None:
    assert path_dialect(parts=test_case.parts, dialects=DEFAULT_DIALECTS) == (
        test_case.expected_dialect
    )


@pytest.mark.parametrize(
    "test_case",
    [
        FolderFeatureTestCase(
            "a folder without its feature is reported",
            "[features]\nsnowflake = []\nduckdb = []\n",
            frozenset({"snowflake", "clickhouse"}),
            ("clickhouse",),
        ),
        FolderFeatureTestCase(
            "every folder with its feature passes",
            "[features]\nsnowflake = []\nclickhouse = []\n",
            frozenset({"snowflake", "clickhouse"}),
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_folders_when_matching_features_then_lists_missing(
    test_case: FolderFeatureTestCase,
) -> None:
    assert folders_without_features(
        folders=test_case.folders, features=read_crate_features(manifest_text=test_case.manifest)
    ) == (test_case.expected_missing)


@pytest.mark.parametrize(
    "test_case",
    [
        SharedLinesTestCase(
            "gated folder module declarations pass",
            '#[cfg(feature = "snowflake")]\npub(crate) mod snowflake;\n'
            '#[cfg(feature = "clickhouse")] mod clickhouse;\nmod names;\n',
            False,
            (),
        ),
        SharedLinesTestCase(
            "an ungated folder module declaration fails",
            "pub mod snowflake;\nmod names;\n",
            False,
            (1,),
        ),
        SharedLinesTestCase(
            "a dialect gate on anything but its folder module fails",
            '#[cfg(feature = "snowflake")]\nfn number_scale() -> u8 {\n    0\n}\n',
            False,
            (1,),
        ),
        SharedLinesTestCase(
            "a shared file that branches on a dialect fails",
            "fn scale(dialect: Dialect) -> u8 {\n    match dialect {\n"
            "        Dialect::Snowflake => 0,\n        _ => 1,\n    }\n}\n",
            False,
            (3,),
        ),
        SharedLinesTestCase(
            "other dialect enums, folder paths and dialect crates count; comments, strings and "
            "the duckdb engine crate do not",
            "// Dialect::Snowflake\n"
            'let s = "crate::snowflake";\n'
            "let d = SourceDialect::ClickHouse;\n"
            "use crate::snowflake::profile;\n"
            "use sqlquack_syntax_duckdb::parse;\n"
            "let c = duckdb::Connection::open_in_memory();\n",
            False,
            (3, 4, 5),
        ),
        SharedLinesTestCase(
            "a /* inside a string does not hide the dialect code after it",
            'let open = "/* not a comment";\n'
            "let d = Dialect::Snowflake;\n"
            'let close = "*/";\n',
            False,
            (2,),
        ),
        SharedLinesTestCase(
            "a // inside a string does not hide the rest of its line",
            'let url = "http://example.com"; let d = Dialect::Snowflake;\n',
            False,
            (1,),
        ),
        SharedLinesTestCase(
            "a quote in a char literal does not open a string",
            "let quote = '\"'; let d = Dialect::Snowflake;\n"
            'let s = "text";\n',
            False,
            (1,),
        ),
        SharedLinesTestCase(
            "nested block comments and raw strings hide what they hold",
            "/* outer /* inner */ Dialect::Snowflake */\n"
            'let s = r#"a "Dialect::Snowflake" b"#;\n',
            False,
            (),
        ),
        SharedLinesTestCase(
            "a registry may name a dialect its own feature gates", REGISTRY, True, ()
        ),
        SharedLinesTestCase(
            "outside a registry the same gated entries fail", REGISTRY, False, (2, 3, 4)
        ),
        SharedLinesTestCase(
            "a registry line gated by another dialect, or not gated, fails",
            "match dialect {\n"
            '    #[cfg(feature = "snowflake")]\n'
            "    Dialect::ClickHouse => Some(&clickhouse::PROFILE),\n"
            "    Dialect::Snowflake => Some(&snowflake::PROFILE),\n"
            "    _ => None,\n"
            "}\n",
            True,
            (3, 4),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_shared_source_when_checking_then_lists_dialect_lines(
    test_case: SharedLinesTestCase,
) -> None:
    found: tuple[SourceLine, ...] = shared_dialect_lines(
        source=test_case.source, vocabulary=VOCABULARY, registry=test_case.registry
    )

    assert tuple(line.line for line in found) == test_case.expected_lines


@pytest.mark.parametrize(
    "test_case",
    [
        ManifestFaultTestCase(
            "a dialect folder without its feature is reported",
            "[features]\ndefault = []\n",
            "has src/snowflake/ but no `snowflake` feature",
        ),
        ManifestFaultTestCase(
            "a dialect in the default features is reported",
            '[features]\ndefault = ["snowflake"]\nsnowflake = []\n',
            "default features turn on `snowflake`",
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_manifest_fault_when_running_fensu_then_reports_it(
    test_case: ManifestFaultTestCase, tmp_path: Path
) -> None:
    crate: Path = tmp_path / "crates" / "sqlquack-semantics"
    (crate / "src" / "snowflake").mkdir(parents=True)
    (tmp_path / "Cargo.toml").write_text(WORKSPACE_MANIFEST)
    (crate / "Cargo.toml").write_text(CRATE_MANIFEST + test_case.features)
    (crate / "src" / "lib.rs").write_text(GATED_MODULE)
    (crate / "src" / "snowflake" / "mod.rs").write_text("pub fn profile() {}\n")
    (tmp_path / "fensu.toml").write_text(FOLDER_RULE_CONFIG)
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
    assert completed.returncode == 1, output
    assert test_case.expected_message in output
    assert "Found 1 fault" in output
