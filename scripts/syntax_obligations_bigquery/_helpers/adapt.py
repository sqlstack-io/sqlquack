"""Point the shared obligation builder at the BigQuery root (see the root README)."""

from __future__ import annotations

import json
import re
import tomllib
from collections import Counter
from collections.abc import Callable
from pathlib import Path

from scripts.grammar_obligations import constants as shared
from scripts.grammar_obligations._helpers import grammar as shared_grammar
from scripts.grammar_obligations._helpers import ids, obligations
from scripts.grammar_obligations.constants import ACTION_HAND, HAND_CASE_KEYS, PAIR_SIZE
from scripts.grammar_obligations.exceptions import ObligationError
from scripts.grammar_obligations.models import Draft, FormStyle, Node, Spec
from scripts.grammar_obligations.types import Key, Row
from scripts.syntax_obligations_bigquery.classes.withdrawals import Withdrawals
from scripts.syntax_obligations_bigquery.constants import (
    BQ_CLASS_RULES,
    BQ_TOKEN,
    CATALOGUE_DIR,
    DISPOSITION_DOCUMENTED,
    DISPOSITION_KEY,
    FAMILIES,
    FAMILY_CONTEXTS,
    FILLS,
    FORM_CLASS_RULES,
    FORM_CONTEXTS,
    FORM_FILLS,
    FORM_LITERAL_FILLS,
    FORM_PREFIX,
    PATTERN_FILLS,
    PLACEHOLDER,
    RETIRED_FILE,
    RETIRED_KIND,
    SUPPRESS_DISPOSITIONS,
)

_shared_existing_ids: Callable[[Path], dict[Key, str]] = ids.existing_ids
_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = shared_grammar.CLASS_RULES
_shared_derive_trees: Callable[..., tuple[list[Draft], list[tuple[Node, ...]], Counter[str]]] = (
    obligations.derive_trees
)
_shared_suppressions: Callable[..., list[Row]] = obligations.suppressions
_shared_suppressed: Callable[..., bool] = obligations._suppressed  # noqa: SLF001


def suppress_rows(*, path: Path, disposition: str) -> list[Row]:
    """The [[suppress]] rows of one disposition; every row must name a known one."""
    rows: list[Row] = _shared_suppressions(path=path)
    unknown: list[str] = [
        f"{row['form']} {row['kind']}"
        for row in rows
        if row.get(DISPOSITION_KEY) not in SUPPRESS_DISPOSITIONS
    ]
    if unknown:
        raise ObligationError(f"[[suppress]] rows with no known disposition: {unknown[:10]}")
    return [row for row in rows if row[DISPOSITION_KEY] == disposition]


def is_suppressed(*, draft: Draft, rows: list[Row]) -> bool:
    """Whether one of the rows names the draft (form, kind, target and note)."""
    return _shared_suppressed(draft=draft, rows=rows)


def catalogue_fills(root: Path) -> dict[str, str]:
    """FILLS, plus a PATTERN_FILLS value for each catalogued placeholder a pattern names."""
    names: set[str] = set()
    for path in sorted((root / CATALOGUE_DIR).glob("*.toml")):
        for form in tomllib.loads(path.read_text()).get("form", []):
            for line in form["grammar"]:
                names |= {name.strip().lower() for name in PLACEHOLDER.findall(str(line))}
    fills: dict[str, str] = {}
    for name in sorted(names - set(FILLS)):
        found: str = next((fill for p, fill in PATTERN_FILLS if p.search(name)), "")
        if found:
            fills[name] = found
    return {**fills, **FILLS}


def _merged(*, form: str, table: dict[str, dict[str, str]]) -> dict[str, str]:
    """The values every key of `table` that prefixes `form` gives, a longer key overriding."""
    merged: dict[str, str] = {}
    for key in sorted((key for key in table if form.startswith(key)), key=len):
        merged.update(table[key])
    return merged


def style(*, form: str, family: str, fills: dict[str, str]) -> FormStyle:
    """The submission style of a family prefix or of one form, by its longest context prefix."""
    owners: list[str] = [key for key in FORM_CONTEXTS if form.startswith(key)]
    context: tuple[str, str] = (
        FORM_CONTEXTS[max(owners, key=len)] if owners else FAMILY_CONTEXTS[family]
    )
    prefix, suffix = context
    return FormStyle(
        prefix=prefix,
        suffix=suffix,
        fills={**fills, **_merged(form=form, table=FORM_FILLS)},
        literal_fills=_merged(form=form, table=FORM_LITERAL_FILLS),
        terminate=False,
    )


def bq_limit_drafts(*, path: Path, family: str, forms: set[str]) -> list[Draft]:
    """The hand-written limits of one family's `bq.` forms, as the shared reader builds them."""
    if not path.exists():
        return []
    drafts: list[Draft] = []
    for row in tomllib.loads(path.read_text()).get("limit", []):
        form: str = str(row["form"])
        if not form.startswith(f"{FORM_PREFIX}.{family}."):
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
                    expect=obligations._limit_expect(row),  # noqa: SLF001
                ),
            )
        )
    return drafts


def retired_ids(root: Path) -> set[str]:
    """The obligation ids the root has retired; they are never issued again."""
    path: Path = root / RETIRED_FILE
    if not path.exists():
        return set()
    groups: list[Row] = tomllib.loads(path.read_text()).get("retired", [])
    retired: set[str] = set()
    for group in groups:
        retired.update(str(identifier) for identifier in group["ids"])
    return retired


def bq_existing_ids(path: Path) -> dict[Key, str]:
    """The kept ids of one obligations file, plus the root's retired ids under keys no draft has."""
    kept: dict[Key, str] = _shared_existing_ids(path)
    for identifier in retired_ids(path.parent.parent):
        kept[(identifier.rsplit(".", 1)[0], RETIRED_KIND, identifier, "", "")] = identifier
    return kept


def _scoped_class_rules(*, form: str) -> tuple[tuple[re.Pattern[str], str], ...]:
    """The FORM_CLASS_RULES entries whose prefix `form` starts with, in their order."""
    scoped: list[tuple[re.Pattern[str], str]] = []
    for prefix, rules in FORM_CLASS_RULES.items():
        if form.startswith(prefix):
            scoped.extend(rules)
    return tuple(scoped)


def bq_derive_trees(
    *, form: str, grammar: list[str]
) -> tuple[list[Draft], list[tuple[Node, ...]], Counter[str]]:
    """The shared derivation, with the class rules of the form's FORM_CLASS_RULES entries first."""
    general: tuple[tuple[re.Pattern[str], str], ...] = shared_grammar.CLASS_RULES
    scoped: tuple[tuple[re.Pattern[str], str], ...] = _scoped_class_rules(form=form)
    shared_grammar.CLASS_RULES = (*scoped, *general)
    try:
        return _shared_derive_trees(form=form, grammar=grammar)
    finally:
        shared_grammar.CLASS_RULES = general


def documented_rows(*, path: Path) -> list[Row]:
    """The [[suppress]] rows that withdraw a draft: those whose text is a documented form."""
    return suppress_rows(path=path, disposition=DISPOSITION_DOCUMENTED)


def install(root: Path) -> Withdrawals:
    """Register the BigQuery styles, classes and readers; the result collects withdrawn drafts."""
    withdrawn: Withdrawals = Withdrawals()
    shared_grammar.CLASS_RULES = (*BQ_CLASS_RULES, *_shared_class_rules)
    fills: dict[str, str] = catalogue_fills(root)
    for family in FAMILIES:
        styles: dict[str, FormStyle] = shared.FAMILY_STYLES.setdefault(family, {})
        styles[f"{FORM_PREFIX}.{family}."] = style(form="", family=family, fills=fills)
        styles.update(
            {
                form: style(form=form, family=family, fills=fills)
                for form in {*FORM_CONTEXTS, *FORM_FILLS, *FORM_LITERAL_FILLS}
                if form.startswith(f"{FORM_PREFIX}.{family}.")
            }
        )
    setattr(shared_grammar, "TOKEN", BQ_TOKEN)  # noqa: B010
    setattr(obligations, "derive_trees", bq_derive_trees)  # noqa: B010
    setattr(obligations, "limit_drafts", bq_limit_drafts)  # noqa: B010
    setattr(obligations, "existing_ids", bq_existing_ids)  # noqa: B010
    setattr(obligations, "suppressions", documented_rows)  # noqa: B010
    setattr(obligations, "_suppressed", withdrawn.suppressed)  # noqa: B010
    return withdrawn
