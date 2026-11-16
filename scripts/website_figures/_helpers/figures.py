"""The figures a coverage summary gives the website, and how the written ones differ."""

from __future__ import annotations

from typing import Any

from scripts.website_figures.constants import BASIS_KEY, FORMS_KEY, PERCENT_KEY, VERIFIED_KEY


def summary_figures(summary: dict[str, Any]) -> dict[str, str]:
    """The verified count, the form count and the verified share, as the website writes them."""
    total: dict[str, int] = summary["total"]
    return {
        VERIFIED_KEY: f"{total['verified']:,}",
        FORMS_KEY: f"{total['forms']:,}",
        PERCENT_KEY: f"{summary['verified_percent']:.2f}%",
    }


def figure_problems(*, figures: dict[str, Any], expected: dict[str, str]) -> list[str]:
    """Each figure that differs from the summary, and a basis that does not state the count."""
    problems: list[str] = [
        f"{key} is {figures.get(key)!r}, but the summary gives {value!r}"
        for key, value in expected.items()
        if figures.get(key) != value
    ]
    basis: object = figures.get(BASIS_KEY)
    if not isinstance(basis, str) or not basis.startswith(f"{expected[VERIFIED_KEY]} "):
        problems.append(
            f"{BASIS_KEY} does not start with the verified count {expected[VERIFIED_KEY]}"
        )
    return problems
