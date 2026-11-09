"""The name checks behind the repository layout rules, against names and sources that differ."""

from __future__ import annotations

import pytest

from scripts.fensu_policy._helpers.layout_names import (
    catalog_spellings,
    crate_name_problem,
    dialect_branches,
    enum_mismatch,
    enum_variants,
    unnamed_defaults,
    unnamed_dependency_dialects,
    variant_map,
)
from scripts.fensu_policy.constants import (
    DEFAULT_DIALECT_VARIANTS,
    DEFAULT_DIALECTS,
    DEFAULT_FRONT_DOORS,
    DEFAULT_ROLES,
)
from scripts.fensu_policy.models import CrateDependencies, SourceLine
from tests.unit.scripts.fensu_policy._test_types import (
    BranchTestCase,
    CrateNameTestCase,
    DependencyDialectTestCase,
    EntriesTestCase,
    EnumTestCase,
)

ALL_DIALECTS: tuple[str, ...] = DEFAULT_DIALECTS
VARIANTS: dict[str, str] = variant_map(entries=DEFAULT_DIALECT_VARIANTS)


@pytest.mark.parametrize(
    "test_case",
    [
        CrateNameTestCase(
            "role then dialect passes",
            "sqlquack-types-snowflake",
            "crates/sqlquack-types-snowflake",
            None,
        ),
        CrateNameTestCase(
            "a source and target pair passes",
            "sqlquack-rules-snowflake-duckdb",
            "crates/sqlquack-rules-snowflake-duckdb",
            None,
        ),
        CrateNameTestCase("the bare product crate passes", "sqlquack", "crates/sqlquack", None),
        CrateNameTestCase(
            "dialect before role fails",
            "sqlquack-snowflake-types",
            "crates/sqlquack-snowflake-types",
            "before the role",
        ),
        CrateNameTestCase(
            "an unknown role fails",
            "sqlquack-widgets",
            "crates/sqlquack-widgets",
            "configured roles",
        ),
        CrateNameTestCase(
            "a non-dialect qualifier fails",
            "sqlquack-syntax-derive",
            "crates/sqlquack-syntax-derive",
            "not a dialect",
        ),
        CrateNameTestCase(
            "the same dialect twice fails",
            "sqlquack-rules-duckdb-duckdb",
            "crates/sqlquack-rules-duckdb-duckdb",
            "more than one",
        ),
        CrateNameTestCase(
            "a folder named differently fails", "sqlquack-lint", "crates/lint", "name the folder"
        ),
        CrateNameTestCase(
            "a tool ending in -check passes",
            "sqlquack-coverage-check",
            "tools/sqlquack-coverage-check",
            None,
        ),
        CrateNameTestCase(
            "a tool without -check fails", "sqlquack-coverage", "tools/sqlquack-coverage", "-check"
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_crate_when_naming_then_reports_problem(test_case: CrateNameTestCase) -> None:
    problem: str | None = crate_name_problem(
        name=test_case.name,
        directory=test_case.directory,
        roles=DEFAULT_ROLES,
        dialects=ALL_DIALECTS,
    )

    assert (problem is None) == (test_case.expected_fragment is None)
    assert test_case.expected_fragment is None or test_case.expected_fragment in str(problem)


@pytest.mark.parametrize(
    "test_case",
    [
        DependencyDialectTestCase(
            "a neutral crate using a parser crate fails",
            "sqlquack-format",
            "crates/sqlquack-format",
            ("sqlquack-syntax-postgres", "sqlquack-syntax"),
            (("sqlquack-syntax-postgres", "postgres"),),
        ),
        DependencyDialectTestCase(
            "a pair crate may use both of its dialects",
            "sqlquack-rules-snowflake-duckdb",
            "crates/sqlquack-rules-snowflake-duckdb",
            ("sqlquack-types-snowflake", "sqlquack-blocks-duckdb"),
            (),
        ),
        DependencyDialectTestCase(
            "a pair crate may not use a third dialect",
            "sqlquack-rules-snowflake-duckdb",
            "crates/sqlquack-rules-snowflake-duckdb",
            ("sqlquack-syntax-sqlite",),
            (("sqlquack-syntax-sqlite", "sqlite"),),
        ),
        DependencyDialectTestCase(
            "a front door may use every dialect",
            "sqlquack-parse",
            "crates/sqlquack-parse",
            ("sqlquack-syntax-bigquery", "sqlquack-syntax-snowflake"),
            (),
        ),
        DependencyDialectTestCase(
            "a development tool may use every dialect",
            "sqlquack-coverage-check",
            "tools/sqlquack-coverage-check",
            ("sqlquack-syntax-sqlserver",),
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_dependencies_when_checking_then_lists_unnamed_dialects(
    test_case: DependencyDialectTestCase,
) -> None:
    found: tuple[tuple[str, str], ...] = unnamed_dependency_dialects(
        crate=CrateDependencies(
            name=test_case.name,
            directory=test_case.directory,
            runtime_dependencies=test_case.runtime_dependencies,
        ),
        dialects=ALL_DIALECTS,
        front_doors=DEFAULT_FRONT_DOORS,
    )

    assert found == test_case.expected_pairs


@pytest.mark.parametrize(
    "test_case",
    [
        EnumTestCase(
            "missing and extra variants are both reported",
            "/// Dialects.\npub enum Dialect {\n    /// Snowflake.\n    Snowflake,\n    #[default]\n    DuckDB,\n}\n",
            ("duckdb", "bigquery"),
            ("BigQuery",),
            ("Snowflake",),
        ),
        EnumTestCase(
            "an exact match reports nothing, and comments naming variants do not count",
            "pub enum Dialect {\n    // SQLite is not here yet\n    DuckDB,\n    PostgreSQL,\n}\n",
            ("duckdb", "postgres"),
            (),
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_enum_when_comparing_with_parsers_then_reports_mismatch(
    test_case: EnumTestCase,
) -> None:
    variants: tuple[str, ...] | None = enum_variants(source=test_case.source, name="Dialect")

    assert variants is not None
    assert enum_mismatch(
        variants=variants, parser_dialects=test_case.parser_dialects, variants_by_dialect=VARIANTS
    ) == (test_case.expected_missing, test_case.expected_extra)


@pytest.mark.parametrize(
    "test_case",
    [
        BranchTestCase(
            "only code lines naming a variant count",
            'fn f(d: Dialect) {\n    // Dialect::Snowflake in a comment\n    let s = "Dialect::DuckDB";\n    match d { Dialect::Snowflake => 1, _ => 2 };\n}\n',
            (4,),
        ),
        BranchTestCase(
            "a generic dialect parameter is not a branch",
            "fn f(dialect: Dialect) -> Dialect { dialect }\n",
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_source_when_finding_branches_then_returns_lines(test_case: BranchTestCase) -> None:
    found: tuple[SourceLine, ...] = dialect_branches(
        source=test_case.source, enum_name="Dialect", variants=tuple(VARIANTS.values())
    )

    assert tuple(line.line for line in found) == test_case.expected_lines


@pytest.mark.parametrize(
    "test_case",
    [
        EntriesTestCase(
            "the unsuffixed sibling of dialect scripts is the unnamed default",
            ("build_syntax_catalogue.py", "build_syntax_catalogue_duckdb.py", "generate_cases.py"),
            ("build_syntax_catalogue.py",),
        ),
        EntriesTestCase(
            "a different job is not an unnamed default",
            ("syntax_receipts", "duckdb_receipts", "sqlite_receipts"),
            (),
        ),
        EntriesTestCase(
            "every sibling named passes",
            ("syntax_catalogue_snowflake", "syntax_catalogue_duckdb"),
            (),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_entries_when_checking_defaults_then_lists_unnamed(
    test_case: EntriesTestCase,
) -> None:
    assert (
        unnamed_defaults(entries=test_case.entries, dialects=ALL_DIALECTS)
        == test_case.expected_entries
    )


@pytest.mark.parametrize(
    "test_case",
    [
        EntriesTestCase(
            "catalog as a word fails, catalogue passes",
            (
                "duckdb_catalog",
                "extract_postgres_catalog.py",
                "syntax_catalogue_duckdb",
                "catalogue_tools",
            ),
            ("duckdb_catalog", "extract_postgres_catalog.py"),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_entries_when_checking_spelling_then_lists_catalog(
    test_case: EntriesTestCase,
) -> None:
    assert catalog_spellings(entries=test_case.entries) == test_case.expected_entries
