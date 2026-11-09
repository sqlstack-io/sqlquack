"""Regenerate the documented PostgreSQL syntax catalogue from the private source cache."""

from __future__ import annotations

from scripts.syntax_catalogue_postgres.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Run the build."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
