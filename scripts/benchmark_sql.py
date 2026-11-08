"""Direct wrapper for the generated-SQL timing benchmark."""

from __future__ import annotations

from scripts.sql_benchmark.main.benchmark import run_sql_benchmark


def main(argv: list[str] | None = None) -> int:
    """Time generated SQL over a million rows against hand-written DuckDB SQL."""
    return run_sql_benchmark(argv)


if __name__ == "__main__":
    raise SystemExit(main())
