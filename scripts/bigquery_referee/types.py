"""Type aliases of the BigQuery referee."""

from __future__ import annotations

from typing import Protocol

from scripts.bigquery_referee.models import Observation

type Row = dict[str, object]
type Request = tuple[str, str, Row | None]
type Observed = tuple[Row, Observation, str]


class Transport(Protocol):
    """One API call: method, path under the API root and JSON body, to the status and reply."""

    def __call__(self, *, method: str, path: str, body: Row | None) -> tuple[int, Row]: ...
