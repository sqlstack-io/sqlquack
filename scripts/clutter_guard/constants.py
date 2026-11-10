"""Clutter guard settings, spelled so that no content pattern matches its own source."""

from __future__ import annotations

PATH_NOT_ALLOWED: str = "path-not-allowed"
AGENT_FILE: str = "agent-file"
PROCESS_NOTE_FILE: str = "process-note-file"
DECISION_REFERENCE: str = "decision-reference"
STREAM_REFERENCE: str = "la" + "ne-reference"
REVIEWER_REFERENCE: str = "audit" + "or-reference"
HOME_PATH: str = "home-path"
SECRET_VARIABLE_NAME: str = "secret-variable-name"
CLUTTER_CLASSES: tuple[str, ...] = (
    PATH_NOT_ALLOWED,
    AGENT_FILE,
    PROCESS_NOTE_FILE,
    DECISION_REFERENCE,
    STREAM_REFERENCE,
    REVIEWER_REFERENCE,
    HOME_PATH,
    SECRET_VARIABLE_NAME,
)

TOP_LEVEL_ENTRIES: frozenset[str] = frozenset(
    {
        ".cargo",
        ".github",
        ".gitignore",
        ".release-please-manifest.json",
        "CHANGELOG.md",
        "Cargo.lock",
        "Cargo.toml",
        "LICENSE",
        "Makefile",
        "README.md",
        "THIRD_PARTY_NOTICES",
        "corpora",
        "coverage",
        "crates",
        "docs",
        "evidence.toml",
        "fensu.toml",
        "fuzz",
        "pyproject.toml",
        "python",
        "recorder",
        "referee",
        "release-please-config.json",
        "rust-toolchain.toml",
        "scripts",
        "sqlquack.toml",
        "suites",
        "tests",
        "tools",
        "uv.lock",
        "website",
    }
)
SUITES_DIRECTORY: str = "suites"
SUITE_FOLDERS: frozenset[str] = frozenset({"syntax", "convert", "referee", "corpora", "measures"})
DIALECT_SUITES: frozenset[str] = frozenset({"syntax", "convert"})
DIALECTS: frozenset[str] = frozenset(
    {
        "snowflake",
        "duckdb",
        "postgres",
        "sqlite",
        "bigquery",
        "databricks",
        "clickhouse",
        "sqlserver",
    }
)

AGENT_NAMES_LOWER_CASE: frozenset[str] = frozenset(
    {"agents.md", "claude.md", ".agents", ".amp", ".claude", ".cursor", ".codex"}
)

PROCESS_NOTE_WORD_PATTERN: str = r"worklist|hand[-_]?off|scratch"
RUN_NOTE_OR_LOG_WORD_PATTERN: str = r"(?<![a-z])run[-_]?(?:note|log)s?(?![a-z])"
REVIEW_RECORD_PREFIX_PATTERN: str = r"^(?:l[a]ne|audit[o]r)[-_]"
PROCESS_NOTE_PATTERNS: tuple[str, ...] = (
    PROCESS_NOTE_WORD_PATTERN,
    RUN_NOTE_OR_LOG_WORD_PATTERN,
    REVIEW_RECORD_PREFIX_PATTERN,
)
RUN_RECORD_NAMES: frozenset[str] = frozenset({"runs.toml"})
SOURCE_FILE_SUFFIXES: tuple[str, ...] = (".rs", ".py")

DECISION_NUMBER_PATTERN: str = r"(?<![A-Za-z0-9])D[0-9]{2,4}(?![A-Za-z0-9])"
LOWER_CASE_DECISION_NAME_PATTERN: str = (
    r"(?<![A-Za-z0-9_.(:/#=-])d[0-9]{2,4}[a-z]?(?=[-_:]|\s+[a-z])"
    r"|(?<=[(])d[0-9]{2,4}(?=-[a-z])"
    r"|(?<=[a-z]-)d[0-9]{2,4}(?![0-9A-Za-z-])"
)
STREAM_WORD_PATTERN: str = r"(?<![A-Za-z0-9])L[A]NES?(?![a-z0-9])|\b[lL](?:ANE|ane)[sS]?\b"
REVIEWER_WORD_PATTERN: str = r"(?<![A-Za-z0-9])[aA][uU][dD][iI][tT][oO][rR][sS]?(?![A-Za-z0-9])"
ABSOLUTE_HOME_PATTERN: str = r"/(?<![\w.~-]/)(?:home|Users)/(?!test/)[A-Za-z0-9._-]+/"
CONTENT_PATTERNS: tuple[tuple[str, str], ...] = (
    (DECISION_REFERENCE, DECISION_NUMBER_PATTERN),
    (DECISION_REFERENCE, LOWER_CASE_DECISION_NAME_PATTERN),
    (STREAM_REFERENCE, STREAM_WORD_PATTERN),
    (REVIEWER_REFERENCE, REVIEWER_WORD_PATTERN),
    (HOME_PATH, ABSOLUTE_HOME_PATTERN),
)

CLASSES_NOT_COUNTED_IN_RECORD_DATA: frozenset[str] = frozenset(
    {DECISION_REFERENCE, STREAM_REFERENCE}
)
SQL_FIELD_VALUE_PATTERN: str = (
    r"(?<![\w-])\"?(?:sql|query|expression)\"?\s*[:=]\s*\"((?:[^\"\\\n]|\\.)*)\""
)
NAME_IDENTIFIER_VALUE_PATTERN: str = r"(?<![\w-])\"?name\"?\s*[:=]\s*\"([A-Za-z0-9_$]*)\""
SETUP_LIST_VALUE_PATTERN: str = (
    r"(?<![\w-])\"?setup\"?\s*[:=]\s*\[((?:[^\]\"\\]|\"(?:[^\"\\]|\\.)*\")*)\]"
)
RECORD_DATA_FIELD_PATTERN: str = (
    f"{SQL_FIELD_VALUE_PATTERN}|{NAME_IDENTIFIER_VALUE_PATTERN}|{SETUP_LIST_VALUE_PATTERN}"
)

VARIABLE_NAME_PATTERN: str = r"[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+"
VARIABLE_NAME_CHARACTERS: frozenset[str] = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")
SECRET_PART_SEARCH_PATTERN: str = r"PAT|TOKEN|SECRET|PASSWORD|PASSWD|API_KEY|ACCESS_KEY|PRIVATE_KEY"
SECRET_PARTS: frozenset[str] = frozenset({"PAT", "TOKEN", "SECRET", "PASSWORD", "PASSWD"})
SECRET_PART_PAIRS: frozenset[tuple[str, str]] = frozenset(
    {("API", "KEY"), ("ACCESS", "KEY"), ("PRIVATE", "KEY")}
)
BEFORE_ENVIRONMENT_READ_PATTERN: str = (
    r"(?:(?<!\w)\$\{?|(?<!\w)\$env:|\bexport\s+|\b(?:secrets|env)\."
    r"|(?:environ\[|environ\.(?:get|pop|setdefault)\(|[gs]etenv\(|delenv\("
    r"|env::var(?:_os)?\(|env!\(|env\[)\s*[\"'])$"
)
BEFORE_LINE_START_ASSIGNMENT_PATTERN: str = r"^\s*(?:export\s+)?$"
AFTER_ASSIGNMENT_PATTERN: str = r"^(?:=|\s*[?:+]=|:\s*[\"']?\$)"
ENVIRONMENT_VARIABLE_PROSE_PATTERN: str = r"(?i)\benv(?:ironment)?[ _-]?var(?:iable)?s?\b"
ALLOWED_SECRET_NAMES: frozenset[str] = frozenset({"GITHUB_TOKEN", "GH_TOKEN"})

BINARY_PROBE_BYTES: int = 8192
SNIPPET_CONTEXT: int = 30
SNIPPET_LIMIT: int = 100
SUITE_FILE_MINIMUM_PARTS: int = 3
DIALECT_SUITE_FILE_MINIMUM_PARTS: int = 4
