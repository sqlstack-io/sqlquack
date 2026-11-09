"""Direct wrapper for writing SQLQuack's evidence in the published evidence layout."""

from __future__ import annotations

from scripts.evidence_publish.main.convert import run_conversion


def main(argv: list[str] | None = None) -> int:
    """Write the published evidence tree and the old to new receipt line hashes."""
    return run_conversion(argv)


if __name__ == "__main__":
    raise SystemExit(main())
