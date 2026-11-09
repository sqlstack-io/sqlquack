"""Regenerate syntax-coverage obligations from the catalogue grammar and the hand-written limits."""

from __future__ import annotations

from scripts.syntax_obligations_snowflake.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Write obligations/<family>.toml for each named family."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
