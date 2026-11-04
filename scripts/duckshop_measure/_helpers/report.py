"""The unit-test report and its text summary."""

from __future__ import annotations

from collections import Counter

from scripts.duckshop_measure.constants import (
    SCORE_EXCLUDED,
    SNOWFLAKE_LABEL,
    TESTS_LABEL,
    TOP_CONSTRUCTS,
)
from scripts.duckshop_measure.models import ModelResult, UnitTestOutcome
from scripts.duckshop_measure.types import ModelOutcome, Outcome


def unit_tests_report(outcomes: list[UnitTestOutcome]) -> dict[str, object]:
    """Counts per outcome, refused constructs and raised guards by test count, and every test."""
    counts: Counter[str] = Counter(outcome.outcome for outcome in outcomes)
    refused: Counter[str] = Counter()
    guards: Counter[str] = Counter()
    for outcome in outcomes:
        refused.update(outcome.details if outcome.outcome == Outcome.REFUSED else ())
        guards.update(outcome.details if outcome.outcome == Outcome.GUARD_RAISED else ())
    return {
        "label": TESTS_LABEL,
        "tests": len(outcomes),
        "outcomes": {name.value: counts[name.value] for name in Outcome},
        "refused_constructs": dict(refused.most_common()),
        "guards": dict(guards.most_common()),
        "results": [
            {"test": outcome.test, "outcome": outcome.outcome, "details": list(outcome.details)}
            for outcome in outcomes
        ],
    }


def unit_tests_summary(outcomes: list[UnitTestOutcome]) -> str:
    """One line of counts, then the most common refused constructs."""
    counts: Counter[str] = Counter(outcome.outcome for outcome in outcomes)
    refused: Counter[str] = Counter()
    for outcome in outcomes:
        refused.update(outcome.details if outcome.outcome == Outcome.REFUSED else ())
    line: str = ", ".join(f"{name.value} {counts[name.value]}" for name in Outcome)
    top: list[str] = [f"  {count} {name}" for name, count in refused.most_common(TOP_CONSTRUCTS)]
    return "\n".join([f"{len(outcomes)} tests ({TESTS_LABEL}): {line}", *top])


def snowflake_report(results: list[ModelResult]) -> dict[str, object]:
    """Counts per outcome, the scored share matching, refused constructs, and every model."""
    counts: Counter[str] = Counter(result.outcome for result in results)
    refused: Counter[str] = Counter()
    for result in results:
        refused.update(result.details if result.outcome == ModelOutcome.REFUSED else ())
    scored: int = sum(count for name, count in counts.items() if name not in SCORE_EXCLUDED)
    by_target: dict[str, Counter[str]] = {}
    for result in results:
        by_target.setdefault(result.target, Counter())[result.outcome.value] += 1
    return {
        "label": SNOWFLAKE_LABEL,
        "models": len(results),
        "scored": scored,
        "outcomes": {name.value: counts[name.value] for name in ModelOutcome},
        "by_target": {target: dict(by_target[target]) for target in sorted(by_target)},
        "refused_constructs": dict(refused.most_common()),
        "results": [
            {
                "model": result.model,
                "target": result.target,
                "outcome": result.outcome,
                "details": list(result.details),
            }
            for result in results
        ],
    }


def snowflake_summary(results: list[ModelResult]) -> str:
    """One line of counts, the scored share, then the most common refused constructs."""
    counts: Counter[str] = Counter(result.outcome for result in results)
    refused: Counter[str] = Counter()
    for result in results:
        refused.update(result.details if result.outcome == ModelOutcome.REFUSED else ())
    scored: int = sum(count for name, count in counts.items() if name not in SCORE_EXCLUDED)
    line: str = ", ".join(f"{name.value} {counts[name.value]}" for name in ModelOutcome)
    top: list[str] = [f"  {count} {name}" for name, count in refused.most_common(TOP_CONSTRUCTS)]
    return "\n".join(
        [
            f"{len(results)} models ({SNOWFLAKE_LABEL}): {line}",
            f"scored {scored}: {counts[ModelOutcome.MATCHES]} match fully, "
            f"{counts[ModelOutcome.MATCHES_AT_MICROSECONDS]} at microsecond precision only; "
            f"{counts[ModelOutcome.DECLARED_DIVERGENCE]} converted with a declared divergence, "
            "not expected to match",
            *top,
        ]
    )
