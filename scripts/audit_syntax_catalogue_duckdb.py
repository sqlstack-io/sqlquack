"""Audit the DuckDB syntax catalogue against the pinned PEG grammar and engine extract."""

from __future__ import annotations

from scripts.syntax_catalogue_duckdb.main.audit import run_audit


def main(argv: list[str] | None = None) -> int:
    """Run the omission audit."""
    return run_audit(argv)


if __name__ == "__main__":
    raise SystemExit(main())
