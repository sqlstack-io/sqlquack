"""Direct wrapper for the syntax receipt import."""

from __future__ import annotations

from scripts.syntax_receipts.main.import_runs import run_import


def main(argv: list[str] | None = None) -> int:
    """Import private Snowflake capture runs as normalized syntax-coverage receipts."""
    return run_import(argv)


if __name__ == "__main__":
    raise SystemExit(main())
