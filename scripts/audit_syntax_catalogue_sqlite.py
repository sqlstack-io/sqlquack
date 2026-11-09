"""Audit the SQLite syntax catalogue against parse.y, the keyword table and the pinned engine."""

from __future__ import annotations

from scripts.syntax_catalogue_sqlite.main.audit import run_audit


def main(argv: list[str] | None = None) -> int:
    """Run the audit."""
    return run_audit(argv)


if __name__ == "__main__":
    raise SystemExit(main())
