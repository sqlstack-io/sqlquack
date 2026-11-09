"""Render syntax-coverage cases from the obligations' case specs."""

from __future__ import annotations

from scripts.syntax_obligations_snowflake.main.cases import run_cases


def main(argv: list[str] | None = None) -> int:
    """Write cases/<family>.jsonl for each named family."""
    return run_cases(argv)


if __name__ == "__main__":
    raise SystemExit(main())
