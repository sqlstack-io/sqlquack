"""Write coverage/syntax/obligations/<family>.toml from the catalogue grammar and limits.toml."""

from __future__ import annotations

import argparse
import json
import tomllib
from pathlib import Path

from scripts.syntax_catalogue._helpers.toml_out import write_tables
from scripts.syntax_obligations._helpers.derive import derive_trees
from scripts.syntax_obligations._helpers.ids import assign, existing_ids
from scripts.syntax_obligations.constants import HAND_CASE_KEYS, KIND_PROSE, STATUS_CATALOGUED
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
                pair=(pair[0], pair[1]) if len(pair) == 2 else (),
                required=bool(row.get("required", True)),
                spec=Spec(
                    "hand",
                    payload=json.dumps(
                        {key: row[key] for key in HAND_CASE_KEYS if key in row}, sort_keys=True
                    ),
                ),
            )
        )
    return drafts


def suppressions(*, path: Path) -> set[tuple[str, str, str]]:
    """(form, kind, target) of derived obligations that limits.toml withdraws, with a reason."""
    if not path.exists():
        return set()
    rows: list[Row] = tomllib.loads(path.read_text()).get("suppress", [])
    if any(not str(row.get("reason", "")).strip() for row in rows):
        raise ObligationError("every [[suppress]] entry in limits.toml needs a reason")
    return {(str(row["form"]), str(row["kind"]), str(row["target"])) for row in rows}


def obligation_rows(numbered: list[tuple[str, Draft]]) -> list[Row]:
    """TOML rows; an interaction's pair names the option_present obligations of its two targets."""
    by_target: dict[tuple[str, str], str] = {
        (draft.form, draft.target): identifier
        for identifier, draft in numbered
        if draft.kind == "option_present"
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
    """The family's numbered obligations (ids kept from its obligations file) and, per form,
    the parsed productions their case specs point into."""
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
        found, trees[str(form["id"])] = derive_trees(str(form["id"]), list(form["grammar"]))
        drafts += found
    suppressed: set[tuple[str, str, str]] = suppressions(path=root / "limits.toml")
    drafts = [d for d in drafts if (d.form, d.kind, d.target) not in suppressed]
    drafts += limit_drafts(
        path=root / "limits.toml", family=family, forms={str(form["id"]) for form in forms}
    )
    drafts.sort(key=lambda draft: draft.form)
    path: Path = root / "obligations" / f"{family}.toml"
    return assign(drafts, existing_ids(path)), trees


def run_build(argv: list[str] | None = None) -> int:
    """Regenerate the obligations of the named families."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax"))
    parser.add_argument("families", nargs="+")
    arguments: argparse.Namespace = parser.parse_args(argv)
    root: Path = arguments.root
    for family in arguments.families:
        path: Path = root / "obligations" / f"{family}.toml"
        numbered, _ = family_obligations(root=root, family=family)
        rows: list[Row] = obligation_rows(numbered)
        path.parent.mkdir(parents=True, exist_ok=True)
        write_tables(
            path=path,
            header=f"# Obligations of family {family}; generated by "
            "scripts/build_syntax_obligations.py from the catalogue grammar and limits.toml.\n"
            "# See docs/syntax-coverage.md.",
            tables=[("obligation", rows)],
        )
    return 0
