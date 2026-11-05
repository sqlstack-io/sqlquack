"""Import private Snowflake capture runs as normalized syntax-coverage receipts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.syntax_receipts._helpers.output import write_import
from scripts.syntax_receipts._helpers.runs import import_runs
from scripts.syntax_receipts.models import Imported
from scripts.syntax_receipts.types import Row


def run_import(argv: list[str] | None = None) -> int:
    """Write receipts for every bound row of the runs, never replacing an existing file."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("runs", type=Path)
    parser.add_argument("receipts", type=Path)
    parser.add_argument("report", type=Path)
    parser.add_argument("inputs", type=Path, nargs="+")
    parser.add_argument("--published", type=Path, nargs="*", default=[])
    arguments: argparse.Namespace = parser.parse_args(argv)
    imported: Imported = import_runs(runs=arguments.runs, input_dirs=arguments.inputs)
    written: Row = write_import(
        imported=imported,
        published=arguments.published,
        receipts_path=arguments.receipts,
        report_path=arguments.report,
    )
    print(json.dumps(written))
    return 0
