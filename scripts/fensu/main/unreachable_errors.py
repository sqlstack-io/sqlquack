"""Entry point: return internal errors where `Result` functions call `unreachable!` (FPRSH003)."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.rewrites.unreachable import unreachable_edits
from scripts.fensu._helpers.rust.comments import apply_edits
from scripts.fensu._helpers.rust.files import rust_files
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.constants import TEST_PATH_PARTS, TESTS_FILE
from scripts.fensu.models import Edit


def run_unreachable_errors(argv: list[str] | None = None) -> int:
    """Rewrite each library `unreachable!` inside a `Result`-returning function as an error."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("unreachable-errors: rewriting Rust sources", file=sys.stderr)
    rewritten: int = 0
    for path in rust_files(roots=arguments.roots):
        if path.name == TESTS_FILE or TEST_PATH_PARTS.intersection(path.parts[:-1]):
            continue
        text: str = path.read_text()
        edits: list[Edit] = unreachable_edits(text=text, tokens=tokenize(text=text))
        rewritten += len(edits)
        if edits:
            path.write_text(apply_edits(text=text, edits=edits))
    print(f"unreachable-errors: done, {rewritten} calls rewritten", file=sys.stderr)
    return 0
