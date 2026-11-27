"""Regenerate the Databricks translation obligations."""

from __future__ import annotations

from scripts.translation_obligations_databricks.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Write coverage/translation-databricks/obligations/<family>.toml from the published cases."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
