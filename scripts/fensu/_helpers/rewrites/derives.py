"""Derive-list edits: add `Debug` to the data models that lack it."""

from __future__ import annotations

import re

from scripts.fensu.constants import DEBUG_DERIVE, DERIVE_PREFIX, ENUM, STRUCT
from scripts.fensu.models import Edit, Item


def debug_edits(*, text: str, items: list[Item]) -> list[Edit]:
    """Edits giving each struct and enum in `items` a derived `Debug`, unless it has one."""
    edits: list[Edit] = []
    for item in items:
        if item.kind not in {STRUCT, ENUM} or _has_debug(text=text, item=item):
            continue
        derive: str | None = next(
            (attribute for attribute in item.attributes if attribute.startswith(DERIVE_PREFIX)),
            None,
        )
        if derive is None:
            edits.append(
                Edit(
                    start=item.header_start,
                    end=item.header_start,
                    replacement=f"#[derive({DEBUG_DERIVE})]\n",
                )
            )
            continue
        at: int = text.index(derive, item.start) + len(DERIVE_PREFIX)
        edits.append(Edit(start=at, end=at, replacement=f"{DEBUG_DERIVE}, "))
    return edits


def _has_debug(*, text: str, item: Item) -> bool:
    derived: bool = any(
        attribute.startswith(DERIVE_PREFIX) and re.search(rf"\b{DEBUG_DERIVE}\b", attribute)
        for attribute in item.attributes
    )
    return derived or re.search(rf"\b{DEBUG_DERIVE}\s+for\s+{item.name}\b", text) is not None
