"""Record BigQuery syntax receipts, compare two runs, or probe one text."""

from __future__ import annotations

from scripts.bigquery_referee.main.run import run_referee


def main(argv: list[str] | None = None) -> int:
    """Run the BigQuery referee."""
    return run_referee(argv)


if __name__ == "__main__":
    raise SystemExit(main())
