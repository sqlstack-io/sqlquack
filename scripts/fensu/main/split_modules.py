"""Entry point: move the syntax crate's items into fensu's domain layout, re-pointing every path."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.fensu._helpers.splitting.emit import write_outputs
from scripts.fensu._helpers.splitting.layout import rewrite_dependents
from scripts.fensu._helpers.splitting.outputs import split_outputs
from scripts.fensu.classes.crate import Crate
from scripts.fensu.classes.splitter import Splitter
from scripts.fensu.constants import DIRECTORY_DOCS, LIB_FILE, PLACEMENTS


def run_split_modules(argv: list[str] | None = None) -> int:
    """Split the syntax crate as the placements say and re-point every path to its items."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    arguments: argparse.Namespace = parser.parse_args(argv)
    repository: Path = arguments.repository
    src: Path = repository / "crates" / "sqlquack-syntax" / "src"
    print("split-modules: reading the syntax crate", file=sys.stderr)
    crate: Crate = Crate(root=src / LIB_FILE)
    splitter: Splitter = Splitter(crate=crate, src=src, placements=PLACEMENTS)
    splitter.plan()
    outputs: dict[str, str] = split_outputs(
        splitter=splitter, crate=crate, src=src, docs=DIRECTORY_DOCS
    )
    write_outputs(src=src, outputs=outputs, removed=splitter.moved_files)
    rewritten: int = rewrite_dependents(splitter=splitter, crate=crate, repository=repository)
    print(
        f"split-modules: done, {len(splitter.destinations)} files written, "
        f"{len(splitter.moved_files)} split, {rewritten} dependent files re-pointed",
        file=sys.stderr,
    )
    return 0
