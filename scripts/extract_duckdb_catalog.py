"""Direct wrapper for the DuckDB function catalogue extract."""

from __future__ import annotations

from scripts.duckdb_catalog.main.extract import run_catalog_extract


def main(argv: list[str] | None = None) -> int:
    """Write DuckDB's function catalogue into the semantics crate."""
    return run_catalog_extract(argv)


if __name__ == "__main__":
    raise SystemExit(main())
