"""Settings for keeping the website's syntax figures equal to the coverage summary."""

from __future__ import annotations

FIGURES_FILE: str = "website/src/data/figures.json"
VERIFIED_KEY: str = "SYNTAX_VERIFIED"
FORMS_KEY: str = "SYNTAX_FORMS"
PERCENT_KEY: str = "SYNTAX_PCT"
BASIS_KEY: str = "SYNTAX_BASIS"
BASIS_TEMPLATE: str = (
    "{verified} forms verified against cases recorded on Snowflake and approved reviews, "
    "as of {day}."
)
WRITE_COMMAND: str = "write"
CHECK_COMMAND: str = "check"
JSON_INDENT: str = "\t"
