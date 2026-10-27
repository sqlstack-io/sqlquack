"""Direct wrapper: move the syntax crate's items into fensu's domain layout."""

from __future__ import annotations

from scripts.fensu.main.split_modules import run_split_modules


def main(argv: list[str] | None = None) -> int:
    """Split the syntax crate's modules and re-point every path to their items."""
    return run_split_modules(argv)


if __name__ == "__main__":
    raise SystemExit(main())
