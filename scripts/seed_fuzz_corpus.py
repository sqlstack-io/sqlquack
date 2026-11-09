"""Direct wrapper for seeding the fuzz corpus."""

from __future__ import annotations

from scripts.fuzz_corpus.main.seed import run_seed


def main(argv: list[str] | None = None) -> int:
    """Seed the fuzz corpus with the exact check's statements."""
    return run_seed(argv)


if __name__ == "__main__":
    raise SystemExit(main())
