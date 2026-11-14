"""The omission audit: every catalogue form either yields obligations or is listed with why not."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.grammar_obligations.constants import KIND_PROSE, STATUS_CATALOGUED
from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_snowflake.constants import (
    ALIASES_FILE,
    AUDIT_ALIAS_TARGETS_EMPTY,
    AUDIT_ALIASED,
    AUDIT_ARITY,
    AUDIT_DERIVED_NONE,
    AUDIT_EMPTY_GRAMMAR,
    AUDIT_FRAGMENT,
    AUDIT_PROSE_UNREAD,
    AUDIT_SUPPRESSED,
    AUDIT_WITHDRAWN,
    CATALOGUE_DIR,
    LIMITS_FILE,
)


def obligated_forms(*, root: Path, family: str) -> set[str]:
    """The forms the committed obligations file of one family gives at least one obligation."""
    path: Path = root / "obligations" / f"{family}.toml"
    if not path.exists():
        return set()
    rows: list[Row] = tomllib.loads(path.read_text()).get("obligation", [])
    return {str(row["id"]).rsplit(".", 1)[0] for row in rows}


def omission_audit(
    *, root: Path, built: dict[str, set[str]], dropped: dict[str, str]
) -> tuple[list[str], list[str]]:
    """Built families' forms without obligations: failures, and those without them by design."""
    obligated: set[str] = set().union(*built.values()) if built else set()
    for family in _families(root):
        if family not in built:
            obligated |= obligated_forms(root=root, family=family)
    limits: Row = _toml(root / LIMITS_FILE)
    readings: set[str] = {str(row["form"]) for row in limits.get("reading", [])}
    suppressed: set[str] = {str(row["form"]) for row in limits.get("suppress", [])}
    aliases: dict[str, list[str]] = {
        str(row["from"]): _targets(row) for row in _toml(root / ALIASES_FILE).get("alias", [])
    }
    failures: list[str] = []
    listed: list[str] = []
    for family in sorted(built):
        for form in _toml(root / CATALOGUE_DIR / f"{family}.toml").get("form", []):
            identifier: str = str(form["id"])
            if identifier in obligated:
                continue
            if form["status"] != STATUS_CATALOGUED:
                targets: list[str] | None = aliases.get(identifier)
                if targets is None:
                    listed.append(f"{identifier}: {AUDIT_FRAGMENT}")
                elif empty := [target for target in targets if target not in obligated]:
                    failures.append(
                        f"{identifier}: {AUDIT_ALIAS_TARGETS_EMPTY}: {', '.join(empty)}"
                    )
                else:
                    listed.append(f"{identifier}: {AUDIT_ALIASED} {', '.join(targets)}")
            elif form["kind"] == KIND_PROSE and identifier not in readings:
                listed.append(f"{identifier}: {AUDIT_PROSE_UNREAD}")
            elif not form["grammar"] and identifier not in readings:
                failures.append(f"{identifier}: {AUDIT_EMPTY_GRAMMAR}")
            else:
                why: str = dropped.get(identifier) or (
                    AUDIT_SUPPRESSED if identifier in suppressed else AUDIT_DERIVED_NONE
                )
                failures.append(f"{identifier}: {why}")
    return failures, listed


def dropped_reasons(*, withdrawn: list[Row], arity: list[Row]) -> dict[str, str]:
    """Why a form may have lost its derived obligations: withdrawals, then arity moves."""
    reasons: dict[str, str] = {str(row["form"]): AUDIT_ARITY for row in arity}
    reasons |= {str(row["form"]): AUDIT_WITHDRAWN for row in withdrawn}
    return reasons


def _targets(alias: Row) -> list[str]:
    """The forms an alias row names as carrying the retired form's grammar."""
    return list(map(str, alias["to"]))


def _families(root: Path) -> list[str]:
    return sorted(path.stem for path in (root / CATALOGUE_DIR).glob("*.toml"))


def _toml(path: Path) -> Row:
    return tomllib.loads(path.read_text()) if path.exists() else {}
