"""How each family's forms become complete submissions; keyed by form-id prefix."""

from __future__ import annotations

from scripts.syntax_obligations._helpers.render import FormStyle

DECLARE_TAIL: str = " BEGIN RETURN 1 ; END ;"
BLOCK_OPEN: str = "BEGIN "
BLOCK_CLOSE: str = " END ;"
CURSOR: str = "{} CURSOR FOR SELECT 1"
RESULTSET: str = "{} RESULTSET DEFAULT ( SELECT 1 )"
ASYNC_RESULTSET: str = "{} RESULTSET DEFAULT ASYNC ( SELECT 1 )"
EXCEPTION: str = "{} EXCEPTION"
VARIABLE: str = "{} INT"
DECLARE_REPEAT: str = (
    "[ { <variable_declaration> | <cursor_declaration> | <resultset_declaration> | "
    "<nested_stored_procedure_declaration> | <exception_declaration> } ; ... ]"
)


def _block(
    *,
    fills: dict[str, str] | None = None,
    clauses: dict[str, str] | None = None,
    uses: dict[str, str] | None = None,
) -> FormStyle:
    return FormStyle(
        prefix=BLOCK_OPEN,
        suffix=BLOCK_CLOSE,
        fills=fills or {},
        clauses=clauses or {},
        uses=uses or {},
    )


def _in_loop() -> FormStyle:
    """BREAK and CONTINUE run inside a loop that carries the label they name."""
    return FormStyle(prefix="BEGIN FOR i0 IN 1 TO 1 DO ", suffix=" END FOR {label} ; END ;")


# Scripting forms run as anonymous blocks, as written, and declare every cursor, result set,
# exception and variable they name, so Snowflake compiles them in full under execute capture.
# Loops get fills that stop them at once: LOOP's statement is BREAK, WHILE's condition is FALSE
# and REPEAT's is TRUE. RAISE runs inside a handler, so a raised exception ends the run cleanly.
SCRIPTING: dict[str, FormStyle] = {
    "sf.scripting.": _block(),
    "sf.scripting.begin": FormStyle(prefix="", suffix=""),
    "sf.scripting.declare.": FormStyle(prefix="DECLARE ", suffix=DECLARE_TAIL),
    "sf.scripting.declare.declare": FormStyle(
        prefix="", suffix=DECLARE_TAIL, clauses={DECLARE_REPEAT: "v2 INT ;"}
    ),
    "sf.scripting.exception": FormStyle(
        prefix="BEGIN NULL ; ",
        suffix=BLOCK_CLOSE,
        fills={"exception_name": "STATEMENT_ERROR"},
        clauses={"[ WHEN ... ]": "WHEN EXPRESSION_ERROR THEN NULL ;"},
        uses={"exception_name": EXCEPTION},
    ),
    "sf.scripting.case.end": _block(clauses={"[ WHEN ... ]": "WHEN 2 THEN NULL ;"}),
    "sf.scripting.case.end.syntax": _block(clauses={"[ WHEN ... ]": "WHEN FALSE THEN NULL ;"}),
    "sf.scripting.loop": _block(fills={"statement": "BREAK"}),
    "sf.scripting.while": _block(fills={"condition": "FALSE"}),
    "sf.scripting.break": _in_loop(),
    "sf.scripting.continue": _in_loop(),
    "sf.scripting.raise": FormStyle(
        prefix=BLOCK_OPEN,
        suffix=" EXCEPTION WHEN OTHER THEN NULL ; END ;",
        uses={"exception_name": EXCEPTION},
    ),
    "sf.scripting.open": _block(
        uses={"cursor_name": CURSOR, "bind_variable_1": VARIABLE, "bind_variable_2": VARIABLE}
    ),
    "sf.scripting.fetch": _block(uses={"cursor_name": CURSOR, "variable": VARIABLE}),
    "sf.scripting.close": _block(uses={"cursor_name": CURSOR}),
    "sf.scripting.for.do-end-for": _block(uses={"cursor_name": CURSOR}),
    "sf.scripting.await": _block(uses={"result_set_name": ASYNC_RESULTSET}),
    "sf.scripting.cancel": _block(uses={"result_set_name": ASYNC_RESULTSET}),
    "sf.scripting.let.let-cursor-for.cursor-assignment-syntax": _block(
        uses={"resultset_name": RESULTSET}
    ),
}

FAMILY_STYLES: dict[str, dict[str, FormStyle]] = {"scripting": SCRIPTING}
