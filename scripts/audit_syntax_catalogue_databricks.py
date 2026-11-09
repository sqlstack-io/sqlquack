"""Run the Databricks catalogue's omission audit."""

from __future__ import annotations

from scripts.syntax_catalogue_databricks.main.audit import run_audit


def main(argv: list[str] | None = None) -> int:
    """Run the audit."""
    return run_audit(argv)


if __name__ == "__main__":
    raise SystemExit(main())
