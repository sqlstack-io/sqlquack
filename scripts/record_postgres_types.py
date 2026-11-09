"""Direct wrapper for recording PostgreSQL result types from a container of the pinned image."""

from __future__ import annotations

from scripts.postgres_catalogue.main.record_types import run_type_recording


def main(argv: list[str] | None = None) -> int:
    """Write the 18.6 server's result type for each matrix expression into the semantics fixture."""
    return run_type_recording(argv)


if __name__ == "__main__":
    raise SystemExit(main())
