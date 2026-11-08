"""Schema qualification of the names a rendered case writes."""

from __future__ import annotations

import re

from scripts.syntax_obligations.constants import (
    DEFINE_REFERENCE_CLAUSES,
    DEFINE_REFERENCE_ITEM,
    DEFINE_SECRETS_CLAUSE,
    DOUBLE_QUOTE,
    SCHEMA_QUALIFIER,
    SINGLE_QUOTE,
)


def qualified_references(sql: str) -> str:
    """DEFINE SQL with the references after its reference clauses and in SECRETS qualified."""
    item: str = DEFINE_REFERENCE_ITEM
    for clause in DEFINE_REFERENCE_CLAUSES:
        sql = re.sub(
            rf"(?<![\w$]){re.escape(clause)} ({item}(?: , {item})*)",
            lambda m: (
                m[0][: len(m[0]) - len(m[1])]
                + " , ".join(_qualified_item(x) for x in m[1].split(" , "))
            ),
            sql,
        )
    return re.sub(
        rf"(?<={re.escape(DEFINE_SECRETS_CLAUSE)} )[^)]*",
        lambda m: re.sub(rf"(?<=' = )({item})", lambda n: _qualified_item(n[1]), m[0]),
        sql,
    )


def _qualified_item(item: str) -> str:
    """A rendered schema-level reference in d1.s1; keywords and qualified names stay as they are."""
    bare: bool = not item.startswith((SINGLE_QUOTE, DOUBLE_QUOTE))
    if item.lower().startswith(("d1.", "'d1.")) or (bare and item == item.upper()):
        return item
    return qualified(prefix=SCHEMA_QUALIFIER, text=item)


def qualified(*, prefix: str, text: str) -> str:
    """`text` behind a database or schema qualifier; inside the quotes of a string-literal name."""
    if not prefix:
        return text
    if text.startswith(SINGLE_QUOTE):
        return f"{SINGLE_QUOTE}{prefix}{text[1:]}"
    return f"{prefix}{text}"
