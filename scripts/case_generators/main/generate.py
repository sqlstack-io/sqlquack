"""Generator entry point: write every generated case file, or check they are up to date."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.case_generators._helpers.registry import generated_texts, stale_files, write_files
from scripts.case_generators.constants import GENERATED_DIRECTORY


def run_generate(argv: list[str] | None = None) -> int:
    """Write the generated case files; with --check, fail when any file differs or is stray."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    arguments: argparse.Namespace = parser.parse_args(argv)
    directory: Path = arguments.repository / GENERATED_DIRECTORY
    texts: dict[str, str] = generated_texts()
    stale: list[str] = stale_files(directory=directory, texts=texts)
    cases: int = sum(text.count("[[case]]") for text in texts.values())
    if arguments.check:
        for name in stale:
            print(f"generated cases: {GENERATED_DIRECTORY}/{name} is out of date", file=sys.stderr)
        print(
            f"generated cases: {'FAILED' if stale else 'passed'}, {len(texts)} files, "
            f"{cases} cases; run `python -m scripts.generate_cases` to regenerate",
            file=sys.stderr,
        )
        return 1 if stale else 0
    write_files(directory=directory, texts=texts)
    print(
        f"generated cases: done, {len(texts)} files, {cases} cases, {len(stale)} changed",
        file=sys.stderr,
    )
    return 0
