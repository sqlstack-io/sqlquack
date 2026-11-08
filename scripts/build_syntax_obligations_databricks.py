"""Regenerate the Databricks obligations and cases."""

from __future__ import annotations

from scripts.syntax_obligations_databricks.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Write obligations/<family>.toml and cases/<family>.jsonl under the Databricks root."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
