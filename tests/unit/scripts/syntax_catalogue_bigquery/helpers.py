"""The committed audit.toml and an audit result that matches it, for the review's guards."""

from __future__ import annotations

import itertools
import tomllib
from pathlib import Path

from scripts.syntax_catalogue_bigquery._helpers.hand import strings
from scripts.syntax_catalogue_bigquery.constants import AUDIT_FILE, CATALOGUE_DIR, UNSEEN_RULES
from scripts.syntax_catalogue_bigquery.models import AuditGap, AuditResult
from scripts.syntax_catalogue_bigquery.types import Review

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-bigquery"
MENTIONED_SPELLINGS: tuple[tuple[str, str], ...] = (
    ("doc.query-syntax", "CREATE RECURSIVE VIEW"),
    ("doc.lexical", "IMPORT MODULE x"),
    ("doc.lexical", "MODULE x"),
    ("doc.migrating-from-legacy-sql", "NATURAL"),
    ("doc.bigqueryml-syntax-create-tflite", "VARIANT"),
)


def committed() -> dict[str, list[dict[str, object]]]:
    """The committed audit.toml."""
    return tomllib.loads((ROOT / AUDIT_FILE).read_text())


def review_without(removed: tuple[tuple[str, str, str], ...]) -> Review:
    """The committed review less each row whose `key` field is `value` in table `table`."""
    tables: dict[str, list[dict[str, object]]] = committed()
    for table, key, value in removed:
        tables[table] = list(filter(lambda row: str(row[key]) != value, tables[table]))
    review: Review = {table: rows for table, rows in tables.items()}
    return review


def result_matching_review(unwalked: frozenset[str]) -> AuditResult:
    """An audit result with exactly the reviewed findings, the `unwalked` rules unreached."""
    review: dict[str, list[dict[str, object]]] = committed()
    gaps: list[AuditGap] = list(itertools.chain.from_iterable(map(gaps_of, review["gap"])))
    spelled: list[tuple[str, list[str]]] = [
        ("doc.test", str(item).replace("…", " ").split())
        for item in itertools.chain(
            spelled_otherwise(rows=review["gap"], field="sequences"),
            spelled_otherwise(rows=review["keyword"], field="keywords"),
        )
    ]
    spelled += [(page, text.split()) for page, text in MENTIONED_SPELLINGS]
    forms: set[str] = set(itertools.chain.from_iterable(map(form_ids, catalogue_paths())))
    keywords: list[str] = list(map(str, items_of(rows=review["keyword"], field="keywords")))
    walked: list[str] = sorted(set(UNSEEN_RULES) - unwalked)
    return AuditResult(
        gaps=gaps,
        derived_gaps=0,
        missing_keywords=keywords,
        dispatched_words=[str(row["word"]) for row in review["dispatched"]],
        missing_functions=[],
        missing_types=[],
        sequences_checked=len(gaps),
        rules_checked=len(walked),
        walked=walked,
        unwalked=[str(row["rule"]) for row in review["unwalked"]],
        keywords_checked=0,
        functions_checked=0,
        types_checked=0,
        features={str(row["name"]): () for row in review["feature"]},
        doc_streams=spelled,
        rejected_alternatives=0,
        statements=frozenset(),
        form_ids=frozenset(forms),
    )


def sequences_of(rule: str) -> list[str]:
    """The committed [[gap]] sequences of one rule, in file order."""
    rows: list[dict[str, object]] = list(
        filter(lambda row: row["rule"] == rule, committed()["gap"])
    )
    return list(map(str, items_of(rows=rows, field="sequences")))


def items_of(*, rows: list[dict[str, object]], field: str) -> list[str]:
    """Every row's list field, chained in file order."""
    return list(itertools.chain.from_iterable(strings(row[field]) for row in rows))


def spelled_otherwise(*, rows: list[dict[str, object]], field: str) -> list[str]:
    """The list field of the rows reviewed as spelled_otherwise."""
    kept: list[dict[str, object]] = list(
        filter(lambda row: row["kind"] == "spelled_otherwise", rows)
    )
    return items_of(rows=kept, field=field)


def gaps_of(row: dict[str, object]) -> list[AuditGap]:
    """One gap per sequence of a [[gap]] row."""
    return [
        AuditGap(rule=str(row["rule"]), alternative=1, keywords=s)
        for s in strings(row["sequences"])
    ]


def catalogue_paths() -> list[Path]:
    """The committed catalogue files."""
    return sorted((ROOT / CATALOGUE_DIR).glob("*.toml"))


def form_ids(path: Path) -> list[str]:
    """The form ids of one catalogue file."""
    return [str(form["id"]) for form in tomllib.loads(path.read_text()).get("form", [])]
