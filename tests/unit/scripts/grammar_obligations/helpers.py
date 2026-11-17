"""Drafts of one form for the obligation row tests."""

from __future__ import annotations

from scripts.grammar_obligations.models import Draft

FORM: str = "db.ddl.f"
OPTION: str = "[ COMMENT <c> ]"
CHOICE: str = "{ ROW FORMAT <r> | STORED AS <s> } → STORED AS <s>"
SHARED: str = "[ LOCATION <p> ]"


def numbered_with(pair: tuple[str, str]) -> list[tuple[str, Draft]]:
    """The form's options and alternatives, then an interaction of `pair`."""
    return [
        (f"{FORM}.o001", Draft(form=FORM, kind="option_present", target=OPTION, note="")),
        (f"{FORM}.o002", Draft(form=FORM, kind="alternative", target=CHOICE, note="")),
        (f"{FORM}.o003", Draft(form=FORM, kind="alternative", target=SHARED, note="")),
        (f"{FORM}.o004", Draft(form=FORM, kind="option_present", target=SHARED, note="")),
        (f"{FORM}.o005", Draft(form=FORM, kind="interaction", target="", note="", pair=pair)),
    ]
