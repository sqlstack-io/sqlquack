"""Direct wrapper for the website figures."""

from __future__ import annotations

from scripts.website_figures.main.figures import run_figures


def main(argv: list[str] | None = None) -> int:
    """Write the website's syntax figures from the coverage summary, or fail when they differ."""
    return run_figures(argv)


if __name__ == "__main__":
    raise SystemExit(main())
