"""Direct wrapper for the PostgreSQL catalogue extract from a container of the pinned image."""

from __future__ import annotations

from scripts.postgres_catalog.main.extract import run_catalog_extract


def main(argv: list[str] | None = None) -> int:
    """Write the pg_proc, pg_cast, pg_type and pg_operator extracts into the semantics crate."""
    return run_catalog_extract(argv)


if __name__ == "__main__":
    raise SystemExit(main())
