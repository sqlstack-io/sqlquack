"""Whether a new parse tree differs from the reviewed one only by the call's set quantifier."""

from __future__ import annotations

from typing import Any

from scripts.rebound_reviews.constants import (
    CALL_NODES,
    EXPLICIT_ALL,
    NEW_FIELD,
    OLD_FIELD,
    QUANTIFIER_OF,
    WRITTEN_ALL,
)


def quantifier_difference(*, old: Any, new: Any, sql: str) -> str | None:
    """None when only call flags became quantifiers, `all` at most as often as SQL has `( ALL`."""
    reason: str | None = _difference(old=old, new=new, path="$", call=False)
    alls: int = _explicit_alls(tree=new, call=False)
    written: int = len(WRITTEN_ALL.findall(sql))
    if reason is None and alls > written:
        reason = f"{alls} calls read as all, but the SQL writes ( ALL {written} times"
    return reason


def _difference(*, old: Any, new: Any, path: str, call: bool) -> str | None:
    """The first difference by JSON path other than a call's flag read as its quantifier."""
    if isinstance(old, dict) and isinstance(new, dict):
        renamed: bool = call and OLD_FIELD in old and NEW_FIELD in new and NEW_FIELD not in old
        reason: str | None = _quantifier(old=old, new=new, path=path) if renamed else None
        keys: list[str] = [NEW_FIELD if renamed and key == OLD_FIELD else key for key in old]
        if reason is None and keys != list(new):
            reason = f"{path}: fields {keys} became {list(new)}"
        for key in keys:
            if reason is None and not (renamed and key == NEW_FIELD):
                reason = _difference(
                    old=old[key], new=new[key], path=f"{path}.{key}", call=key in CALL_NODES
                )
        return reason
    if isinstance(old, list) and isinstance(new, list):
        if len(old) != len(new):
            return f"{path}: {len(old)} items became {len(new)}"
        for index, (before, after) in enumerate(zip(old, new, strict=True)):
            reason = _difference(old=before, new=after, path=f"{path}[{index}]", call=False)
            if reason is not None:
                return reason
        return None
    if type(old) is not type(new) or old != new:
        return f"{path}: {old!r} became {new!r}"
    return None


def _quantifier(*, old: dict[str, Any], new: dict[str, Any], path: str) -> str | None:
    """None when a call's flag reads as its quantifier: true as distinct, false as none or all."""
    flag: Any = old[OLD_FIELD]
    quantifier: Any = new[NEW_FIELD]
    if not isinstance(flag, bool):
        return f"{path}.{OLD_FIELD}: {flag!r} is not a flag"
    if quantifier == QUANTIFIER_OF[flag] or (quantifier == EXPLICIT_ALL and flag is False):
        return None
    return f"{path}: {OLD_FIELD} {flag} became {NEW_FIELD} {quantifier!r}"


def _explicit_alls(*, tree: Any, call: bool) -> int:
    """How many calls in `tree` record an explicit `all` quantifier."""
    if isinstance(tree, dict):
        own: int = int(call and tree.get(NEW_FIELD) == EXPLICIT_ALL)
        return own + sum(
            _explicit_alls(tree=value, call=key in CALL_NODES) for key, value in tree.items()
        )
    if isinstance(tree, list):
        return sum(_explicit_alls(tree=value, call=False) for value in tree)
    return 0
