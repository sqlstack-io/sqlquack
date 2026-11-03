"""Import private Snowflake capture runs as normalized syntax-coverage receipts.

Usage: import_syntax_receipts.py RUNS RECEIPTS REPORT INPUT_DIR [INPUT_DIR ...]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from scripts.syntax_receipts.main.import_runs import run_import


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if len(args) < 4:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    runs, receipts, report, *inputs = (Path(arg) for arg in args)
    print(json.dumps(run_import(runs, inputs, receipts, report)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
