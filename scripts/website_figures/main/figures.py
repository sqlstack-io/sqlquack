"""Write the website's syntax figures from a coverage summary, or check that they match it."""

from __future__ import annotations

import argparse
import datetime
import json
import sys
from pathlib import Path
from typing import Any

from scripts.website_figures._helpers.figures import figure_problems, summary_figures
from scripts.website_figures.constants import (
    BASIS_KEY,
    BASIS_TEMPLATE,
    CHECK_COMMAND,
    FIGURES_FILE,
    FORMS_KEY,
    JSON_INDENT,
    VERIFIED_KEY,
    WRITE_COMMAND,
)


def run_figures(argv: list[str] | None = None) -> int:
    """Write or check the syntax figures against `syntax-coverage summary --json` output."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=run_figures.__doc__)
    parser.add_argument("command", choices=(WRITE_COMMAND, CHECK_COMMAND))
    parser.add_argument("summary", type=Path, help="the JSON `syntax-coverage summary` printed")
    parser.add_argument("--figures", type=Path, default=Path(FIGURES_FILE))
    parser.add_argument("--day", type=datetime.date.fromisoformat, default=datetime.date.today())
    arguments: argparse.Namespace = parser.parse_args(argv)
    expected: dict[str, str] = summary_figures(json.loads(arguments.summary.read_text()))
    figures: dict[str, Any] = json.loads(arguments.figures.read_text(encoding="utf-8"))
    if arguments.command == WRITE_COMMAND:
        day: datetime.date = arguments.day
        basis: str = BASIS_TEMPLATE.format(
            verified=expected[VERIFIED_KEY], day=f"{day.day} {day:%B %Y}"
        )
        figures.update({**expected, BASIS_KEY: basis})
        text: str = json.dumps(figures, indent=JSON_INDENT, ensure_ascii=False) + "\n"
        arguments.figures.write_text(text, encoding="utf-8")
        print(f"website figures: wrote {basis}")
        return 0
    problems: list[str] = figure_problems(figures=figures, expected=expected)
    for problem in problems:
        print(f"website figures: {problem}", file=sys.stderr)
    if problems:
        print(
            f"website figures: rewrite them with `make website-figures-update`, then commit "
            f"{FIGURES_FILE}",
            file=sys.stderr,
        )
        return 1
    print(f"website figures: passed, {expected[VERIFIED_KEY]} of {expected[FORMS_KEY]} forms")
    return 0
