"""Entry point: derive `Debug` on every data model (fensu FPRSS201) in the given Rust sources."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.rewrites.derives import debug_edits
from scripts.fensu._helpers.rust.comments import apply_edits
from scripts.fensu._helpers.rust.files import rust_files
from scripts.fensu._helpers.rust.items import parse_items
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.constants import MODELS_FILE
from scripts.fensu.models import Edit


def run_derive_debug(argv: list[str] | None = None) -> int:
    """Add `Debug` to the derives of each struct and enum in the `models.rs` files under roots."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("derive-debug: rewriting data models", file=sys.stderr)
    added: int = 0
    for path in rust_files(roots=arguments.roots):
        if path.name != MODELS_FILE:
            continue
        text: str = path.read_text()
        edits: list[Edit] = debug_edits(
            text=text,
            items=parse_items(text=text, tokens=tokenize(text=text), start=0, end=len(text)),
        )
        added += len(edits)
        if edits:
            path.write_text(apply_edits(text=text, edits=edits))
    print(f"derive-debug: done, {added} models given Debug", file=sys.stderr)
    return 0
