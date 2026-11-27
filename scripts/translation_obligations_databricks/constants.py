"""Paths, kinds and texts of the Databricks translation obligation builder."""

from __future__ import annotations

from pathlib import Path

DEFAULT_CATALOGUE: Path = Path("coverage/syntax-databricks/catalogue")
DEFAULT_OUT: Path = Path("coverage/translation-databricks/obligations")
TRANSLATION_DIR: str = "translation"
CASES_PREFIX: str = "cases"
LIVE_STATUS: str = "catalogued"
KINDS: dict[str, tuple[str, str]] = {
    "type-combination": (
        "type_combination",
        "the argument types read another way, or the result typed otherwise",
    ),
    "null": (
        "null_argument",
        "a NULL argument returning a value, or raising where Databricks returns NULL",
    ),
    "error-parity": (
        "error_parity",
        "a value returned where Databricks rejects the arguments, or another error kind",
    ),
}
READING: str = "the argument types the generator read from the function's page"
NO_INPUT: str = "no input"
HEADER: str = (
    "# Translation obligations of family {family}: per form, the generated cases' type\n"
    "# combinations, NULL arguments and error parity, each met by its one case. Written by\n"
    "# scripts/build_translation_obligations_databricks.py from the published cases' tags;\n"
    "# boundary cases are uncited. See README.md.\n"
)
