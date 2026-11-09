"""PostgreSQL catalogue type aliases."""

from __future__ import annotations

from scripts.syntax_catalogue_postgres.models import Section

type Row = dict[str, object]
type Spans = list[tuple[int, int, tuple[Section, ...]]]
