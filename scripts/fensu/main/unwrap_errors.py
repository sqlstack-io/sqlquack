"""Entry point: propagate internal errors where `Result` functions unwrap options (FPRSH010)."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.rewrites.unwraps import unwrap_edits
from scripts.fensu._helpers.rust.comments import apply_edits
from scripts.fensu._helpers.rust.files import rust_files
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.constants import TEST_PATH_PARTS, TESTS_FILE
from scripts.fensu.models import Edit


def run_unwrap_errors(argv: list[str] | None = None) -> int:
    """Rewrite each library `.unwrap()`/`.expect(..)` in a `Result` function as an error."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("unwrap-errors: rewriting Rust sources", file=sys.stderr)
    rewritten: int = 0
    for path in rust_files(roots=arguments.roots):
        if path.name == TESTS_FILE or TEST_PATH_PARTS.intersection(path.parts[:-1]):
            continue
        text: str = path.read_text()
        edits: list[Edit] = unwrap_edits(text=text, tokens=tokenize(text=text))
        rewritten += len(edits)
        if edits:
            path.write_text(apply_edits(text=text, edits=edits))
    print(f"unwrap-errors: done, {rewritten} calls rewritten", file=sys.stderr)
    return 0
