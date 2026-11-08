"""Build the Databricks SQL syntax catalogue from the private page cache."""

from __future__ import annotations

from scripts.syntax_catalogue_databricks.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Run the build."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
