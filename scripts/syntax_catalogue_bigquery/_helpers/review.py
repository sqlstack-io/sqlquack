"""Compare the omission audit with audit.toml, checking each reviewed claim against the docs."""

from __future__ import annotations

from pathlib import Path

from scripts.grammar_catalogue._helpers.grammar import sha256
from scripts.syntax_catalogue_bigquery._helpers.audit import documented_at
from scripts.syntax_catalogue_bigquery.constants import (
    ABSENT_KINDS,
    FEATURE_ENABLED,
    FEATURE_NOT_ENABLED,
    FEATURE_STATES,
    GAP_KINDS,
    GOOGLESQL_ONLY,
    GOOGLESQL_ONLY_REASON,
    GRAMMAR_DIR,
    GRAMMAR_FILE,
    KEYWORDS_FILE,
    MENTIONED_ON,
    OUTSIDE_PINNED,
    OUTSIDE_PINNED_REASON,
    SPELLED_OTHERWISE,
    UNSEEN_RULES,
)
from scripts.syntax_catalogue_bigquery.models import AuditResult
from scripts.syntax_catalogue_bigquery.types import Review


def source_hashes(cache: Path) -> dict[str, str]:
    """The review keys of the pinned grammar and keyword files."""
    return {
        "grammar_sha256": sha256((cache / GRAMMAR_DIR / GRAMMAR_FILE).read_text()),
        "keywords_sha256": sha256((cache / GRAMMAR_DIR / KEYWORDS_FILE).read_text()),
    }


def rows(*, review: Review, table: str) -> list[dict[str, object]]:
    """A review table's rows, or none."""
    found: object = review.get(table, [])
    return [row for row in found if isinstance(row, dict)] if isinstance(found, list) else []


def listed(*, row: dict[str, object], key: str) -> list[str]:
    """A row's list field as strings."""
    value: object = row.get(key, [])
    return [str(item) for item in value] if isinstance(value, list) else []


def listed_all(*, rows: list[dict[str, object]], key: str) -> set[str]:
    """Every row's list field, as one set of strings."""
    found: set[str] = set()
    for row in rows:
        found |= set(listed(row=row, key=key))
    return found


def reviewed_gaps(review: Review) -> set[str]:
    """Every `rule: keywords` the [[gap]] rows review."""
    found: set[str] = set()
    for row in rows(review=review, table="gap"):
        found |= {f"{row['rule']}: {sequence}" for sequence in listed(row=row, key="sequences")}
    return found


def compare(*, kind: str, current: set[str], reviewed: set[str]) -> list[str]:
    """Unreviewed current items and reviews that name nothing current."""
    return [f"unreviewed {kind}: {item}" for item in sorted(current - reviewed)] + [
        f"{kind} review names nothing current: {item}" for item in sorted(reviewed - current)
    ]


def kind_problems(*, label: str, row: dict[str, object], spelled: dict[str, str]) -> list[str]:
    """A row's kind against the docs: absent kinds spell nothing, spelled_otherwise spells some."""
    kind: str = str(row.get("kind", ""))
    reason: str = str(row.get("reason", ""))
    problems: list[str] = []
    if kind not in GAP_KINDS:
        return [f"{label}: unknown kind {kind!r}"]
    if kind == GOOGLESQL_ONLY and not reason.startswith(GOOGLESQL_ONLY_REASON):
        problems.append(f"{label}: a {GOOGLESQL_ONLY} reason starts {GOOGLESQL_ONLY_REASON!r}")
    if kind == OUTSIDE_PINNED and not reason.startswith(OUTSIDE_PINNED_REASON):
        problems.append(f"{label}: an {OUTSIDE_PINNED} reason starts {OUTSIDE_PINNED_REASON!r}")
    mentioned: set[str] = set(listed(row=row, key=MENTIONED_ON))
    if kind in ABSENT_KINDS:
        problems += [
            f"{label}: reviewed {kind}, but {page} spells {item}"
            for item, page in sorted(spelled.items())
            if page and page not in mentioned
        ]
    problems += [
        f"{label}: {MENTIONED_ON} names {page}, which spells none of it"
        for page in sorted(mentioned - set(spelled.values()))
    ]
    if kind == SPELLED_OTHERWISE and not any(spelled.values()):
        problems.append(f"{label}: reviewed {SPELLED_OTHERWISE}, but no page spells it")
    return problems


def gap_problems(*, result: AuditResult, review: Review) -> list[str]:
    """Unreviewed and stale gaps, kinds the docs contradict, and unnamed language features."""
    current: dict[str, tuple[str, ...]] = {
        f"{gap.rule}: {gap.keywords}": gap.features for gap in result.gaps
    }
    problems: list[str] = compare(kind="gap", current=set(current), reviewed=reviewed_gaps(review))
    disabled: set[str] = {
        str(row["name"])
        for row in rows(review=review, table="feature")
        if row.get("bigquery") == FEATURE_NOT_ENABLED
    }
    for row in rows(review=review, table="gap"):
        sequences: list[str] = listed(row=row, key="sequences")
        statement: bool = str(row["rule"]) in result.statements
        spelled: dict[str, str] = {
            sequence: documented_at(
                sequence=sequence, streams=result.doc_streams, leading=statement
            )
            for sequence in sequences
        }
        problems += kind_problems(label=f"gap {row['rule']}", row=row, spelled=spelled)
        gated: set[str] = set()
        for sequence in sequences:
            gated |= set(current.get(f"{row['rule']}: {sequence}", ())) & disabled
        problems += [
            f"gap {row['rule']}: its reason does not name {feature}, which gates it"
            for feature in sorted(gated)
            if feature not in str(row.get("reason", ""))
        ]
    return problems


def keyword_problems(*, result: AuditResult, review: Review) -> list[str]:
    """Keywords no catalogued grammar spells: each reviewed, and its kind true to the docs."""
    keyword_rows: list[dict[str, object]] = rows(review=review, table="keyword")
    problems: list[str] = compare(
        kind="keyword",
        current=set(result.missing_keywords),
        reviewed=listed_all(rows=keyword_rows, key="keywords"),
    )
    for row in keyword_rows:
        spelled: dict[str, str] = {
            word: documented_at(sequence=word, streams=result.doc_streams, leading=False)
            for word in listed(row=row, key="keywords")
        }
        problems += kind_problems(
            label=f"keyword row {listed(row=row, key='keywords')[:3]}", row=row, spelled=spelled
        )
    return problems


def feature_problems(*, result: AuditResult, review: Review) -> list[str]:
    """Every language feature the grammar's actions test, reviewed as enabled or not."""
    feature_rows: list[dict[str, object]] = rows(review=review, table="feature")
    problems: list[str] = compare(
        kind="language feature",
        current=set(result.features),
        reviewed={str(row["name"]) for row in feature_rows},
    )
    for row in feature_rows:
        state: str = str(row.get("bigquery", ""))
        if state not in FEATURE_STATES:
            problems.append(f"feature {row['name']}: unknown state {state!r}")
        if state == FEATURE_ENABLED:
            problems += [
                f"feature {row['name']}: evidence {form} is not a catalogued form"
                for form in listed(row=row, key="evidence")
                if form not in result.form_ids
            ]
            if not listed(row=row, key="evidence"):
                problems.append(f"feature {row['name']}: enabled with no catalogued evidence")
        if state == FEATURE_NOT_ENABLED and not str(row.get("reason", "")).startswith(
            GOOGLESQL_ONLY_REASON
        ):
            problems.append(f"feature {row['name']}: its reason starts {GOOGLESQL_ONLY_REASON!r}")
    return problems


def problems_of(*, result: AuditResult, review: Review, hashes: dict[str, str]) -> list[str]:
    """Every unreviewed or stale finding of the audit; none means the review is complete."""
    problems: list[str] = [
        f"{key} is {review.get(key)!r}, the pinned source is {value}"
        for key, value in hashes.items()
        if review.get(key) != value
    ]
    problems += gap_problems(result=result, review=review)
    problems += keyword_problems(result=result, review=review)
    problems += compare(
        kind="dispatched word",
        current=set(result.dispatched_words),
        reviewed={str(row["word"]) for row in rows(review=review, table="dispatched")},
    )
    problems += compare(
        kind="function",
        current=set(result.missing_functions),
        reviewed={str(row["name"]) for row in rows(review=review, table="function")},
    )
    problems += compare(
        kind="type",
        current=set(result.missing_types),
        reviewed={str(row["name"]) for row in rows(review=review, table="type")},
    )
    problems += feature_problems(result=result, review=review)
    problems += compare(
        kind="unseen rule",
        current=set(UNSEEN_RULES),
        reviewed={str(row["rule"]) for row in rows(review=review, table="unseen")},
    )
    problems += [
        f"unseen rule the walk does not reach: {rule}"
        for rule in UNSEEN_RULES
        if rule not in result.walked
    ]
    problems += compare(
        kind="unwalked rule",
        current=set(result.unwalked),
        reviewed={str(row["rule"]) for row in rows(review=review, table="unwalked")},
    )
    return problems


def audit_summary(*, result: AuditResult, review: Review, problems: list[str]) -> dict[str, object]:
    """The counts the audit prints."""
    return {
        "rules": result.rules_checked,
        "rules_unwalked": len(result.unwalked),
        "alternatives_rejected": result.rejected_alternatives,
        "unseen_reviewed": len(rows(review=review, table="unseen")),
        "keyword_sequences": result.sequences_checked,
        "gaps_reviewed": len(result.gaps),
        "gaps_derived": result.derived_gaps,
        "language_features": len(result.features),
        "keywords": result.keywords_checked,
        "keywords_not_catalogued": len(result.missing_keywords),
        "dispatched_not_catalogued": len(result.dispatched_words),
        "functions": result.functions_checked,
        "functions_not_catalogued": len(result.missing_functions),
        "types": result.types_checked,
        "types_not_catalogued": len(result.missing_types),
        "problems": len(problems),
    }
