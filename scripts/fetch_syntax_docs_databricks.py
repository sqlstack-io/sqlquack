"""Fetch the pinned Databricks SQL language reference and Spark grammar into a private cache."""

from __future__ import annotations

from scripts.syntax_catalogue_databricks.main.fetch import run_fetch


def main(argv: list[str] | None = None) -> int:
    """Run the fetch."""
    return run_fetch(argv)


if __name__ == "__main__":
    raise SystemExit(main())
