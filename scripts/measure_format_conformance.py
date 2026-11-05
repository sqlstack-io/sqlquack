"""Direct wrapper for the formatter conformance measurement against SQLClean."""

from __future__ import annotations

from scripts.format_conformance.main.compare import run_format_conformance


def main(argv: list[str] | None = None) -> int:
    """Measure how often `sqlquack convert --pretty` lays SQL out as `sqlclean format` does."""
    return run_format_conformance(argv)


if __name__ == "__main__":
    raise SystemExit(main())
