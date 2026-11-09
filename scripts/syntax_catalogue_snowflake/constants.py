"""Constants of syntax_catalogue_snowflake."""

from __future__ import annotations

import re

REFERENCE_ROOT: str = "sql-reference"

COMMANDS_DIRECTORY: str = "sql"

CALL_DIRECTORIES: frozenset[str] = frozenset({"functions", "stored-procedures"})

CLASSES_DIRECTORY: str = "classes"

SCRIPTING_DIRECTORY: str = "snowflake-scripting"

CONSTRUCTS_DIRECTORY: str = "constructs"

VIEW_DIRECTORIES: frozenset[str] = frozenset({"info-schema", "account-usage", "organization-usage"})

QUERY_PAGES: frozenset[str] = frozenset({"operators-query", "constructs"})

WINDOW_SYNTAX_PREFIX: str = "functions-window-syntax"

CONSTRAINTS_PREFIX: str = "constraints"

LOG_EVERY: int = 200

FORMAT_MARKDOWN: str = "markdown"

FORMAT_HTML: str = "html"

NAV_INDEX_SUFFIX: str = ".json"

FETCH_MIN_ARGUMENTS: int = 2

FETCH_USAGE: str = (
    "Usage: python -m scripts.fetch_syntax_docs_snowflake CACHE INDEX_FILE [INDEX_FILE ...]"
)

SOURCE_FIELD: str = "source"


PAGE_FORMS: str = "forms"

PAGE_NO_SYNTAX: str = "no_syntax_block"

PAGE_OUT_OF_SCOPE: str = "out_of_scope"

PAGE_RESTATED: str = "restated"

DECISION_MERGED_HEADINGS: str = "identical grammar under several headings merged into one form"

FOLD_IDENTICAL_REASON: str = (
    "grammar repeated byte for byte on another page; "
    "counted once under the command's own page (D80)"
)

DECISION_PROSE: str = "syntax heading '{heading}' has no syntax block; catalogued from prose"

LEADING_EXTRA: frozenset[str] = frozenset(
    {"DESC", "DESCRIBE", "LS", "RM", "WITH", "SELECT", "START", "BEGIN"}
)

DEFINITION_QUALIFIER: re.Pattern[str] = re.compile(r"\S+ (\([^()]*\)) ::=")

ADMIN_WORDS: re.Pattern[str] = re.compile(
    r"\b(GRANT\w*|REVOKE|USERS?|ROLES?|POLIC(Y|IES)|INTEGRATIONS?|ACCOUNTS?|ORGANIZATION\w*|"
    r"CONNECTIONS?|FAILOVER GROUPS?|REPLICATION GROUPS?|RESOURCE MONITORS?|SECRETS?|SHARES?|"
    r"PRIVILEGES?|NETWORK RULES?)\b",
    re.I,
)

INDEX_MARKDOWN_URL: re.Pattern[str] = re.compile(r"https://docs\.snowflake\.com/en/[^) \s]*\.md")
