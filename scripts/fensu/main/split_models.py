"""Entry point: split the expression models into one domain per area, re-pointing every path."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.splitting.emit import write_outputs
from scripts.fensu._helpers.splitting.layout import rewrite_dependents
from scripts.fensu._helpers.splitting.outputs import split_outputs
from scripts.fensu.classes.crate import Crate
from scripts.fensu.classes.splitter import Splitter
from scripts.fensu.constants import (
    EXPRESSIONS_MOD,
    LIB_FILE,
    MOD_DECLARATION_PREFIXES,
    MODEL_AREA_DOCS,
    MODEL_PLACEMENTS,
)


def run_split_models(argv: list[str] | None = None) -> int:
    """Move each area's models into its own domain and declare the areas in `expressions`."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    arguments: argparse.Namespace = parser.parse_args(argv)
    repository: Path = arguments.repository
    src: Path = repository / "crates" / "sqlquack-syntax" / "src"
    print("split-models: reading the syntax crate", file=sys.stderr)
    crate: Crate = Crate(root=src / LIB_FILE)
    splitter: Splitter = Splitter(crate=crate, src=src, placements=MODEL_PLACEMENTS)
    splitter.plan()
    outputs: dict[str, str] = split_outputs(
        splitter=splitter, crate=crate, src=src, docs=MODEL_AREA_DOCS
    )
    outputs[EXPRESSIONS_MOD] = _declare_areas(text=outputs[EXPRESSIONS_MOD])
    write_outputs(src=src, outputs=outputs, removed=splitter.moved_files)
    rewritten: int = rewrite_dependents(splitter=splitter, crate=crate, repository=repository)
    print(
        f"split-models: done, {len(splitter.destinations)} model files written, "
        f"{rewritten} dependent files re-pointed",
        file=sys.stderr,
    )
    return 0


def _declare_areas(*, text: str) -> str:
    lines: list[str] = text.rstrip("\n").split("\n")
    prefixes: tuple[str, ...] = MOD_DECLARATION_PREFIXES
    declarations: set[str] = {line for line in lines if line.startswith(prefixes)}
    declarations.update(f"pub mod {directory.split('/')[-1]};" for directory in MODEL_AREA_DOCS)
    first: int = next(index for index, line in enumerate(lines) if line.startswith(prefixes))
    last: int = max(index for index, line in enumerate(lines) if line.startswith(prefixes))
    ordered: list[str] = sorted(declarations, key=lambda line: line.split(" mod ")[1])
    return "\n".join([*lines[:first], *ordered, *lines[last + 1 :]]) + "\n"
