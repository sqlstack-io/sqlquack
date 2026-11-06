"""Audit the PostgreSQL syntax catalogue against the pinned gram.y and kwlist.h."""

from __future__ import annotations

from scripts.syntax_catalogue_pg.main.audit import run_audit


def main(argv: list[str] | None = None) -> int:
    """Run the omission audit."""
    return run_audit(argv)


if __name__ == "__main__":
    raise SystemExit(main())
