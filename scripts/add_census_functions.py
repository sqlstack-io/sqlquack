"""Direct wrapper: census-only function signatures from SQLBuild's compiled function files."""

from __future__ import annotations

from scripts.census_functions.main.add_functions import run_add_functions


def main(argv: list[str] | None = None) -> int:
    """Add a functions section to a census schema file."""
    return run_add_functions(argv)


if __name__ == "__main__":
    raise SystemExit(main())
