"""Writing SQLQuack's recorded evidence out in the evidence repository's layout."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.evidence_publish._helpers.layout import copy_evidence, publish_syntax
from scripts.evidence_publish.constants import (
    GITIGNORE_FILE,
    GITIGNORE_TEXT,
    HASH_MAP_FILE,
    SYNTAX_DIALECTS,
)


def run_conversion(argv: list[str] | None = None) -> int:
    """Write SQLQuack's evidence in the published layout, and the old to new receipt hashes."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--out", type=Path, required=True, help="an empty or absent directory")
    parser.add_argument("--hash-map", type=Path, required=True, help=f"where {HASH_MAP_FILE} goes")
    arguments: argparse.Namespace = parser.parse_args(argv)
    out: Path = arguments.out
    if out.exists() and any(out.iterdir()):
        print(f"publish: {out} is not empty", file=sys.stderr)
        return 2
    mapping: dict[str, dict[str, str]] = {
        dialect: publish_syntax(repository=arguments.repository, dialect=dialect, out=out)
        for dialect in SYNTAX_DIALECTS
    }
    copy_evidence(repository=arguments.repository, out=out)
    (out / GITIGNORE_FILE).write_text(GITIGNORE_TEXT)
    arguments.hash_map.write_text(json.dumps(mapping, indent=1, sort_keys=True) + "\n")
    files: int = sum(1 for path in out.rglob("*") if path.is_file())
    print(f"publish: wrote {files} files to {out}; receipt hashes in {arguments.hash_map}")
    return 0
