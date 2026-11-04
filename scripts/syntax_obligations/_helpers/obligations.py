"""Number a family's derived and hand-written obligations and lay them out as TOML rows."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

from scripts.syntax_obligations._helpers.derive import derive_trees
from scripts.syntax_obligations._helpers.ids import assign, existing_ids
from scripts.syntax_obligations.constants import (
    ACTION_HAND,
    EXPECT_ACCEPT,
    EXPECT_REJECT,
    HAND_CASE_KEYS,
    KIND_NEGATIVE,
    KIND_OPTION_PRESENT,
    KIND_PROSE,
    LATER_ERROR_STAGES,
    PAIR_SIZE,
    STATUS_CATALOGUED,
)
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, Node, Spec
from scripts.syntax_obligations.types import Row


def limit_drafts(*, path: Path, family: str, forms: set[str]) -> list[Draft]:
    """The hand-written documented limits of one family, in file order."""
    if not path.exists():
        return []
    drafts: list[Draft] = []
    for row in tomllib.loads(path.read_text()).get("limit", []):
        form: str = str(row["form"])
        if not form.startswith(f"sf.{family}."):
            continue
        if form not in forms:
            raise ObligationError(f"limit on {form}, which is not a catalogued form")
        pair: list[str] = [str(member) for member in row.get("pair", [])]
        drafts.append(
            Draft(
                form=form,
                kind=str(row["kind"]),
                target=str(row.get("target", "")),
                note=str(row["note"]),
                anchor=str(row.get("anchor", "")),
                depths=tuple(int(depth) for depth in row.get("depths", [])),
                pair=(pair[0], pair[1]) if len(pair) == PAIR_SIZE else (),
                required=bool(row.get("required", True)),
                spec=Spec(
                    ACTION_HAND,
                    payload=json.dumps(
                        {key: row[key] for key in HAND_CASE_KEYS if key in row}, sort_keys=True
                    ),
                    expect=_limit_expect(row),
                ),
            )
        )
    return drafts


def _limit_expect(row: Row) -> str:
    """A limit's `expect`, else reject for a negative and accept otherwise; boundaries must say."""
    kind: str = str(row["kind"])
    expect: str = str(row.get("expect", ""))
    if not expect and kind.startswith("boundary_"):
        raise ObligationError(f"limit {row['form']} {kind}: a boundary needs an explicit expect")
    for key in ("stage", "also_stage"):
        if str(row.get(key, "")) not in LATER_ERROR_STAGES:
            raise ObligationError(f"limit {row['form']}: unknown {key} {row[key]!r}")
    return expect or (EXPECT_REJECT if kind == KIND_NEGATIVE else EXPECT_ACCEPT)


def suppressions(*, path: Path) -> list[Row]:
    """Derived obligations limits.toml withdraws with a reason, by form, kind, target and note."""
    if not path.exists():
        return []
    rows: list[Row] = tomllib.loads(path.read_text()).get("suppress", [])
    if any(not str(row.get("reason", "")).strip() for row in rows):
        raise ObligationError("every [[suppress]] entry in limits.toml needs a reason")
    return rows


def _suppressed(*, draft: Draft, rows: list[Row]) -> bool:
    return any(
        (draft.form, draft.kind, draft.target) == (row["form"], row["kind"], row.get("target", ""))
        and row.get("note", draft.note) == draft.note
        for row in rows
    )


def obligation_rows(numbered: list[tuple[str, Draft]]) -> list[Row]:
    """TOML rows; an interaction's pair names the option_present obligations of its two targets."""
    by_target: dict[tuple[str, str], str] = {
        (draft.form, draft.target): identifier
        for identifier, draft in numbered
        if draft.kind == KIND_OPTION_PRESENT
    }
    rows: list[Row] = []
    seen: set[str] = set()
    for identifier, draft in numbered:
        if identifier in seen:
            raise ObligationError(f"{identifier} is derived twice")
        seen.add(identifier)
        row: Row = {"id": identifier, "kind": draft.kind, "required": draft.required}
        if draft.target:
            row["target"] = draft.target
        if draft.depths:
            row["depths"] = list(draft.depths)
        if draft.pair:
            members: list[str] = [by_target.get((draft.form, member), "") for member in draft.pair]
            if not all(members):
                raise ObligationError(
                    f"{identifier}: pair {draft.pair} is not two options of {draft.form}"
                )
            row["pair"] = members
        if draft.anchor:
            row["anchor"] = draft.anchor
        if draft.note:
            row["note"] = draft.note
        rows.append(row)
    return rows


def family_obligations(
    *, root: Path, family: str
) -> tuple[list[tuple[str, Draft]], dict[str, list[tuple[Node, ...]]]]:
    """The family's numbered obligations (kept ids) and each form's parsed productions."""
    catalogue: list[Row] = tomllib.loads((root / "catalogue" / f"{family}.toml").read_text())[
        "form"
    ]
    forms: list[Row] = [
        form
        for form in catalogue
        if form["status"] == STATUS_CATALOGUED and form["kind"] != KIND_PROSE
    ]
    drafts: list[Draft] = []
    trees: dict[str, list[tuple[Node, ...]]] = {}
    for form in forms:
        found, trees[str(form["id"])] = derive_trees(
            form=str(form["id"]), grammar=list(form["grammar"])
        )
        drafts += found
    suppressed: list[Row] = suppressions(path=root / "limits.toml")
    drafts = [d for d in drafts if not _suppressed(draft=d, rows=suppressed)]
    drafts += limit_drafts(
        path=root / "limits.toml", family=family, forms={str(form["id"]) for form in forms}
    )
    drafts.sort(key=lambda draft: draft.form)
    path: Path = root / "obligations" / f"{family}.toml"
    return assign(drafts=drafts, kept=existing_ids(path)), trees
