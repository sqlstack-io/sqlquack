"""Record SQLite 3.54.1 syntax receipts, or compare two runs."""

from __future__ import annotations

from scripts.sqlite_referee.main.run import run_referee


def main(argv: list[str] | None = None) -> int:
    """Run the SQLite referee."""
    return run_referee(argv)


if __name__ == "__main__":
    raise SystemExit(main())
