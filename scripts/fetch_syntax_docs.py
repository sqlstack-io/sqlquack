"""Fetch the Snowflake documentation pages the syntax catalogue pins into a private cache."""

from __future__ import annotations

from scripts.syntax_catalogue.main.fetch import run_fetch


def main(argv: list[str] | None = None) -> int:
    """Fetch every page listed in the llms.txt indexes and navigation trees given."""
    return run_fetch(argv)


if __name__ == "__main__":
    raise SystemExit(main())
