"""Direct wrapper: how often SQLQuack's divergence guards raise on sampled source rows."""

from __future__ import annotations

from scripts.guard_measurement.main.measure import run_measure


def main(argv: list[str] | None = None) -> int:
    """Measure guard raises over Parquet samples."""
    return run_measure(argv=argv)


if __name__ == "__main__":
    raise SystemExit(main())
