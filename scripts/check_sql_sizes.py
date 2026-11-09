"""Direct wrapper for the generated-SQL size ratchet."""

from __future__ import annotations

from scripts.sql_sizes.main.check import run_sql_size_check


def main(argv: list[str] | None = None) -> int:
    """Fail when the size baseline is stale or generated SQL grew against the base branch."""
    return run_sql_size_check(argv)


if __name__ == "__main__":
    raise SystemExit(main())
