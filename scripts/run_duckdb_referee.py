"""Record DuckDB 2.0.0 syntax receipts, or compare two runs."""

from __future__ import annotations

from scripts.duckdb_referee.main.run import run_referee


def main(argv: list[str] | None = None) -> int:
    """Run the DuckDB referee."""
    return run_referee(argv)


if __name__ == "__main__":
    raise SystemExit(main())
