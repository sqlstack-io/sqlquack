"""Databricks referee type aliases."""

from __future__ import annotations

from scripts.databricks_receipts.models import Failure, Observation

type Row = dict[str, object]
type Judged = tuple[Row, Observation | Failure, Row, str]
type Snapshot = dict[str, dict[str, Row]]
