"""Give drafts stable obligation ids, keeping the ids of an existing obligations file."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_obligations.constants import ID_WIDTH, PAIR_SEPARATOR
from scripts.syntax_obligations.models import Draft
from scripts.syntax_obligations.types import Key, Row


def _key(*, form: str, kind: str, target: str, note: str, pair: str) -> Key:
    return (form, kind, target, note, pair)


def existing_ids(path: Path) -> dict[Key, str]:
    """Keys of the obligations already in `path`, with their ids; pairs are keyed by target."""
    if not path.exists():
        return {}
    rows: list[Row] = tomllib.loads(path.read_text()).get("obligation", [])
    target_of: dict[str, str] = {str(row["id"]): str(row.get("target", "")) for row in rows}
    kept: dict[Key, str] = {}
    for row in rows:
        identifier: str = str(row["id"])
        pair: str = PAIR_SEPARATOR.join(
            target_of.get(str(member), "") for member in row.get("pair", [])
        )
        form: str = identifier.rsplit(".", 1)[0]
        kept[
            _key(
                form=form,
                kind=str(row["kind"]),
                target=str(row.get("target", "")),
                note=str(row.get("note", "")),
                pair=pair,
            )
        ] = identifier
    return kept


def assign(*, drafts: list[Draft], kept: dict[Key, str]) -> list[tuple[str, Draft]]:
    """Each draft with its id: the kept id for a known key, else the form's next free number."""
    used: dict[str, int] = {}
    for identifier in kept.values():
        form, number = identifier.rsplit(".o", 1)
        used[form] = max(used.get(form, 0), int(number))
    result: list[tuple[str, Draft]] = []
    for draft in drafts:
        key: Key = _key(
            form=draft.form,
            kind=draft.kind,
            target=draft.target,
            note=draft.note,
            pair=PAIR_SEPARATOR.join(draft.pair),
        )
        identifier: str | None = kept.get(key)
        if identifier is None:
            used[draft.form] = used.get(draft.form, 0) + 1
            identifier = f"{draft.form}.o{used[draft.form]:0{ID_WIDTH}d}"
        result.append((identifier, draft))
    return result
