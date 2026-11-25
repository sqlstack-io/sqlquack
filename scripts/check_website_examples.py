"""Direct wrapper for the website examples."""

from __future__ import annotations

from scripts.website_examples.main.examples import run_examples


def main(argv: list[str] | None = None) -> int:
    """Check the website's examples against the real output of the built `sqlquack`."""
    return run_examples(argv)


if __name__ == "__main__":
    raise SystemExit(main())
