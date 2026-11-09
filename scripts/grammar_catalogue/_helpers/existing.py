"""The ids and forms a catalogue already holds, which a rebuild keeps stable."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.grammar_catalogue.constants import STATUS_ABSENT
from scripts.grammar_catalogue.models import Existing
from scripts.grammar_catalogue.types import Row


def load_existing(out: Path) -> Existing:
    """The ids the current catalogue and aliases.toml have already used."""
    forms: dict[str, Row] = {}
    by_key: dict[tuple[str, str], str] = {}
    by_source: dict[str, list[str]] = {}
    catalogue: Path = out / "catalogue"
    for path in sorted(catalogue.glob("*.toml")) if catalogue.exists() else []:
        for form in tomllib.loads(path.read_text()).get("form", []):
            forms[form["id"]] = form
            by_key[(form["source"], form["grammar_sha256"])] = form["id"]
            by_source.setdefault(form["source"], []).append(form["id"])
    ids: set[str] = set(forms)
    aliases: Path = out / "aliases.toml"
    if aliases.exists():
        ids |= {a["from"] for a in tomllib.loads(aliases.read_text()).get("alias", [])}
    return Existing(forms=forms, by_key=by_key, by_source=by_source, ids=ids)


def absent_forms(*, existing: Existing, emitted: set[str]) -> list[Row]:
    """Earlier forms whose grammar no longer appears, kept with status absent_from_source."""
    return [
        dict(form, status=STATUS_ABSENT)
        for form_id, form in existing.forms.items()
        if form_id not in emitted
    ]
