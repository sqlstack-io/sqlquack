"""Direct wrapper: census-only function signatures from SQLBuild's compiled function files."""

from __future__ import annotations

from scripts.census_functions.main.add_functions import run_add_functions


def main(argv: list[str] | None = None) -> int:
    """Write a census project folder's function files."""
    return run_add_functions(argv)


if __name__ == "__main__":
    raise SystemExit(main())
