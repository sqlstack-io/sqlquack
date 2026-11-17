"""Direct wrapper for the re-bound review check."""

from __future__ import annotations

from scripts.rebound_reviews.main.check import run_rebound_check


def main(argv: list[str] | None = None) -> int:
    """Fail when a re-bound review's form changed by more than its class allows.

    A set_quantifier re-bind may differ only by the call flag read as its quantifier in the trees.
    A stage re-bind may differ only by the stage of cases expected to be accepted, each the stage
    of its receipt phase; their setup, session, SQL text and obligations are taken from
    --old-cases, compared with the current cases field by field.
    """
    return run_rebound_check(argv)


if __name__ == "__main__":
    raise SystemExit(main())
