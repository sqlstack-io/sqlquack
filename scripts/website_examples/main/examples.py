"""Run every example the website shows and fail where a page differs from the real output."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.website_examples._helpers.blocks import shown_blocks
from scripts.website_examples._helpers.running import block_problems
from scripts.website_examples.constants import EXAMPLES_DIR


def run_examples(argv: list[str] | None = None) -> int:
    """Check each marked example block against its example's real output; 1 when any differs."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=run_examples.__doc__)
    parser.add_argument("sqlquack", type=Path, help="the built sqlquack binary")
    parser.add_argument("--root", type=Path, default=Path())
    arguments: argparse.Namespace = parser.parse_args(argv)
    blocks, unmarked = shown_blocks(arguments.root)
    problems: list[str] = list(unmarked)
    for block in blocks:
        problems += block_problems(block=block, root=arguments.root, sqlquack=arguments.sqlquack)
    for problem in problems:
        print(f"website examples: {problem}", file=sys.stderr)
    if problems:
        print(
            f"website examples: update the page to the real output, or the inputs in "
            f"{EXAMPLES_DIR}/, then run `make website-examples`",
            file=sys.stderr,
        )
        return 1
    print(f"website examples: passed, {len(blocks)} blocks")
    return 0
