"""Direct wrapper for the re-bound review check."""

from __future__ import annotations

from scripts.rebound_reviews.main.check import run_rebound_check


def main(argv: list[str] | None = None) -> int:
    """Fail when a re-bound review's form changed by more than its class allows."""
    return run_rebound_check(argv)


if __name__ == "__main__":
    raise SystemExit(main())
