"""Direct wrapper for the referee comparison report."""

from __future__ import annotations

from scripts.referee_tools.main.report import run_report


def main(argv: list[str] | None = None) -> int:
    """Compare SQLQuack's parse with Snowflake's recorded referee outcomes."""
    return run_report(argv)


if __name__ == "__main__":
    raise SystemExit(main())
