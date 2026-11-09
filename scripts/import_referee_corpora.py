"""Direct wrapper for the referee corpus import."""

from __future__ import annotations

from scripts.referee_tools.main.corpora import run_corpora


def main(argv: list[str] | None = None) -> int:
    """Write referee corpus files from a compiled project or sqlglot's tests."""
    return run_corpora(argv)


if __name__ == "__main__":
    raise SystemExit(main())
