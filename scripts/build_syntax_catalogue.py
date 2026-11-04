"""Regenerate the documented Snowflake syntax catalogue from the private page cache.

Usage: build_syntax_catalogue.py --cache CACHE [--out coverage/syntax]
"""

from __future__ import annotations

from scripts.syntax_catalogue.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Write sources.toml, indexes.toml and catalogue/<family>.toml."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
