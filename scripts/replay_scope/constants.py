"""The paths that decide what a change replays and checks; README.md explains each list."""

from __future__ import annotations

FULL_REPLAY_PATHS: tuple[str, ...] = (
    "Cargo.lock",
    "rust-toolchain.toml",
    "evidence.toml",
    "uv.lock",
    ".cargo/config.toml",
    ".github/workflows/ci.yml",
    "crates/sqlquack-compare/",
    "crates/sqlquack-diagnostics/src/failures/",
    "crates/sqlquack-diagnostics/src/provenance/",
    "crates/sqlquack-diagnostics/src/snowflake/failures/",
    "crates/sqlquack-evidence/",
    "crates/sqlquack-policy/",
    "crates/sqlquack/src/cli/_helpers/verify_command.rs",
    "scripts/replay_scope/",
)
REPLAY_INPUT_PATHS: tuple[str, ...] = (
    "Cargo.toml",
    "Makefile",
    "coverage/sql-size/",
    "coverage/translation/",
    "scripts/check_sql_sizes.py",
    "scripts/sql_sizes/",
    "tools/sqlquack-coverage-check/",
)
DOCUMENTATION_PATHS: tuple[str, ...] = ("website/", "docs/")
SITE_PATHS: tuple[str, ...] = (
    "website/",
    ".github/workflows/website-build.yml",
    "rust-toolchain.toml",
)
SITE_MANIFEST: str = "website/wasm/Cargo.toml"
BINARY_PACKAGE: str = "sqlquack"
BUILD_TABLES: tuple[str, ...] = ("dependencies", "build-dependencies")
PATH_KEY: str = "path"
DIRECTORY_SUFFIX: str = "/"
MARKDOWN_SUFFIX: str = ".md"
SCOPE_COMMAND: str = "scope"
DIGEST_COMMAND: str = "digest"
CHECK_COMMAND: str = "check"
