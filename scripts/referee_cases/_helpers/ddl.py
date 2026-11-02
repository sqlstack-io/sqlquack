"""The setup statements creating an environment's empty tables."""

from __future__ import annotations

from scripts.referee_cases.models import Environment, Table, TableColumn


def setup_sql(environment: Environment) -> tuple[str, ...]:
    """One `CREATE TEMPORARY TABLE` per table, in the environment's order."""
    return tuple(_create(table) for table in environment.tables)


def _create(table: Table) -> str:
    return f"CREATE TEMPORARY TABLE {table.name} ({', '.join(map(_column, table.columns))})"


def _column(column: TableColumn) -> str:
    suffix: str = "" if column.nullable else " NOT NULL"
    return f"{column.name} {column.data_type}{suffix}"
