"""How each family's forms become complete submissions; keyed by form-id prefix."""

from __future__ import annotations

from scripts.syntax_obligations._helpers.render import FormStyle

DECLARE_TAIL: str = " BEGIN RETURN 1 ; END ;"

# Scripting forms run as anonymous blocks, as written. Loops get fills that stop them at once:
# LOOP's statement is BREAK, WHILE's condition is FALSE and REPEAT's is TRUE.
SCRIPTING: dict[str, FormStyle] = {
    "sf.scripting.": FormStyle(prefix="BEGIN ", suffix=" END ;"),
    "sf.scripting.begin": FormStyle(prefix="", suffix=""),
    "sf.scripting.declare.": FormStyle(prefix="DECLARE ", suffix=DECLARE_TAIL),
    "sf.scripting.declare.declare": FormStyle(prefix="", suffix=DECLARE_TAIL),
    "sf.scripting.exception": FormStyle(
        prefix="BEGIN NULL ; ",
        suffix=" END ;",
        fills={"exception_name": "STATEMENT_ERROR"},
        clauses={"[ WHEN ... ]": "WHEN EXPRESSION_ERROR THEN NULL ;"},
    ),
    "sf.scripting.case.end": FormStyle(
        prefix="BEGIN ", suffix=" END ;", clauses={"[ WHEN ... ]": "WHEN 2 THEN NULL ;"}
    ),
    "sf.scripting.case.end.syntax": FormStyle(
        prefix="BEGIN ", suffix=" END ;", clauses={"[ WHEN ... ]": "WHEN FALSE THEN NULL ;"}
    ),
    "sf.scripting.loop": FormStyle(prefix="BEGIN ", suffix=" END ;", fills={"statement": "BREAK"}),
    "sf.scripting.while": FormStyle(prefix="BEGIN ", suffix=" END ;", fills={"condition": "FALSE"}),
}

FAMILY_STYLES: dict[str, dict[str, FormStyle]] = {"scripting": SCRIPTING}
