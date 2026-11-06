"""Regenerate the PostgreSQL obligations and the focus forms' cases."""

from __future__ import annotations

from scripts.syntax_obligations_pg.main.build import run_build


def main(argv: list[str] | None = None) -> int:
    """Write obligations/<family>.toml and cases/<family>.jsonl under the Postgres root."""
    return run_build(argv)


if __name__ == "__main__":
    raise SystemExit(main())
