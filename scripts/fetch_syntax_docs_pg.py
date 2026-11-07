"""Fetch the pinned PostgreSQL 18.6 release into a private source cache."""

from __future__ import annotations

from scripts.syntax_catalogue_pg.main.fetch import run_fetch


def main(argv: list[str] | None = None) -> int:
    """Run the fetch."""
    return run_fetch(argv)


if __name__ == "__main__":
    raise SystemExit(main())
