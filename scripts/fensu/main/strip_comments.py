"""Entry point: remove non-doc comments (fensu FPRSH002) from Rust sources."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.rust.comments import apply_edits, comment_edits
from scripts.fensu._helpers.rust.files import rust_files
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.models import Edit


def run_strip_comments(argv: list[str] | None = None) -> int:
    """Rewrite every Rust file under the given roots without non-doc comments."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("strip-comments: rewriting Rust sources", file=sys.stderr)
    removed: int = 0
    for path in rust_files(roots=arguments.roots):
        text: str = path.read_text()
        edits: list[Edit] = comment_edits(text=text, tokens=tokenize(text=text))
        removed += len(edits)
        if edits:
            path.write_text(apply_edits(text=text, edits=edits))
    print(f"strip-comments: done, {removed} comments removed", file=sys.stderr)
    return 0
