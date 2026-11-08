"""Write coverage/syntax-duckdb obligations and cases, or with --check compare them."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import tempfile
import tomllib
from pathlib import Path

from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import ReceiptView
from scripts.syntax_obligations_duckdb._helpers.adapt import install, retired_ids
from scripts.syntax_obligations_duckdb._helpers.cases import (
    receipt_view,
    stale_files,
    write_family,
)
from scripts.syntax_obligations_duckdb.constants import (
    DEFAULT_ROOT,
    FAMILIES,
    OBLIGATIONS_DIR,
    RECEIPTS_DIR,
)


def _issued_ids(root: Path) -> set[str]:
    """Every obligation id written under the root's obligations/."""
    issued: set[str] = set()
    for family in FAMILIES:
        path: Path = root / OBLIGATIONS_DIR / f"{family}.toml"
        if not path.exists():
            continue
        rows: list[dict[str, object]] = tomllib.loads(path.read_text()).get("obligation", [])
        issued.update(str(row["id"]) for row in rows)
    return issued


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate every family's obligations and cases, or with --check compare them."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument(
        "--check", action="store_true", help="write nothing; fail when a committed file differs"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    install()
    if arguments.check:
        return _check(arguments.root)
    _build(arguments.root)
    return 0


def _build(root: Path) -> None:
    """Write every family's obligations and cases into the root; fail if an id vanished."""
    previous: set[str] = _issued_ids(root)
    receipts: ReceiptView = receipt_view(root / RECEIPTS_DIR)
    for family in FAMILIES:
        obligations, cases = write_family(root=root, family=family, receipts=receipts)
        print(json.dumps({"family": family, "obligations": obligations, "cases": cases}))
    retired: set[str] = retired_ids(root)
    issued: set[str] = _issued_ids(root)
    reissued: set[str] = retired & issued
    if reissued:
        raise ObligationError(f"retired obligation ids issued again: {sorted(reissued)}")
    vanished: set[str] = previous - issued - retired
    if vanished:
        raise ObligationError(
            f"obligation ids left the root without a tombstone: {sorted(vanished)}"
        )


def _check(root: Path) -> int:
    """Build a copy of the root; print each committed file that differs, and fail when any does."""
    with tempfile.TemporaryDirectory() as directory:
        built: Path = Path(directory) / "root"
        shutil.copytree(root, built)
        _build(built)
        stale: list[Path] = stale_files(root=root, built=built)
    for path in stale:
        print(f"stale: {path} is missing, differs, or is not written by the build", file=sys.stderr)
    print(f"dialect cases: {root}: {len(stale)} stale files; run without --check to regenerate")
    return 1 if stale else 0
