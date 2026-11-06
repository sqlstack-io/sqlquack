"""Direct wrapper for the basics corpus generator."""

from __future__ import annotations

from scripts.basics_corpus.main.generate import run_generate


def main(argv: list[str] | None = None) -> int:
    """Write corpora/basics/queries.toml with each statement's recorded outcome."""
    return run_generate(argv)


if __name__ == "__main__":
    raise SystemExit(main())
