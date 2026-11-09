"""Record Databricks receipts for case files, or compare two runs."""

from __future__ import annotations

from scripts.databricks_receipts.main.run import run_referee


def main(argv: list[str] | None = None) -> int:
    """Run the referee."""
    return run_referee(argv)


if __name__ == "__main__":
    raise SystemExit(main())
