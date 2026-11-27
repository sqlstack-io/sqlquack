"""Write the Databricks translation obligations from the cases, or with --check compare them."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.translation_obligations_databricks._helpers.obligations import (
    family_entries,
    family_text,
    live_forms,
)
from scripts.translation_obligations_databricks.constants import DEFAULT_CATALOGUE, DEFAULT_OUT
from scripts.translation_obligations_databricks.types import Entry


def _check(*, out: Path, texts: dict[Path, str]) -> int:
    """Fail on a committed file the build writes otherwise, or does not write."""
    stale: list[str] = [
        path.as_posix()
        for path, text in texts.items()
        if not path.exists() or path.read_text() != text
    ]
    extra: list[str] = [path.as_posix() for path in sorted(out.glob("*.toml")) if path not in texts]
    for path in stale + extra:
        print(f"translation obligations: {path} differs from the build", file=sys.stderr)
    return 1 if stale or extra else 0


def run_build(argv: list[str] | None = None) -> int:
    """Write every family's obligations, or with --check fail when a committed file differs."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", type=Path, required=True, help="the published cases' root")
    parser.add_argument("--catalogue", type=Path, default=DEFAULT_CATALOGUE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument(
        "--check", action="store_true", help="write nothing; fail when a committed file differs"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    families: dict[str, list[Entry]] = family_entries(
        cases=arguments.cases, live=live_forms(catalogue=arguments.catalogue)
    )
    texts: dict[Path, str] = {
        arguments.out / f"{family}.toml": family_text(family=family, entries=entries)
        for family, entries in sorted(families.items())
    }
    if arguments.check:
        return _check(out=arguments.out, texts=texts)
    arguments.out.mkdir(parents=True, exist_ok=True)
    for path, text in texts.items():
        path.write_text(text)
        print(json.dumps({"file": path.as_posix(), "obligations": text.count("[[obligation]]")}))
    return 0
