"""Which workspace crates may reach polyglot: only the check crate, as docs/syntax.md explains."""

from __future__ import annotations

POLYGLOT_PACKAGE: str = "polyglot-sql"
CHECK_CRATE: str = "sqlquack-polyglot-check"
ALLOWED_DEPENDENTS: frozenset[str] = frozenset({CHECK_CRATE})
