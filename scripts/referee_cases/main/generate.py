"""Referee generator entry point: write every generated referee file, or check them."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.referee_cases._helpers.registry import generated_texts, stale_files, write_files
from scripts.referee_cases.constants import REFEREE_DIRECTORY, STATEMENT_LINE_PREFIX


def run_generate(argv: list[str] | None = None) -> int:
    """Write the generated referee files; with --check, fail when any differs or is stray."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    arguments: argparse.Namespace = parser.parse_args(argv)
    directory: Path = arguments.repository / REFEREE_DIRECTORY
    texts: dict[str, str] = generated_texts()
    stale: list[str] = stale_files(directory=directory, texts=texts)
    statements: int = sum(text.count(STATEMENT_LINE_PREFIX) for text in texts.values())
    if arguments.check:
        for name in stale:
            print(f"referee cases: {REFEREE_DIRECTORY}/{name} is out of date", file=sys.stderr)
        print(
            f"referee cases: {'FAILED' if stale else 'passed'}, {len(texts)} files, "
            f"{statements} statements; run `python -m scripts.generate_referee_cases` to "
            "regenerate",
            file=sys.stderr,
        )
        return 1 if stale else 0
    write_files(directory=directory, texts=texts)
    print(
        f"referee cases: done, {len(texts)} files, {statements} statements, {len(stale)} changed",
        file=sys.stderr,
    )
    return 0
