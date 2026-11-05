"""Direct wrapper: DuckShop's census schema, and its unit tests converted and run on DuckDB."""

from __future__ import annotations

from scripts.duckshop_measure.main.measure import run_measure


def main(argv: list[str] | None = None) -> int:
    """Run one DuckShop measure."""
    return run_measure(argv=argv)


if __name__ == "__main__":
    raise SystemExit(main())
