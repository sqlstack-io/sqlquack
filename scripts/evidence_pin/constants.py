"""Evidence pin settings."""

from __future__ import annotations

from pathlib import Path

PIN_FILE: str = "evidence.toml"
PIN_KEYS: frozenset[str] = frozenset({"repository", "commit", "content_sha256"})
COMMIT_PATTERN: str = r"[0-9a-f]{40}"
CONTENT_SHA256_PATTERN: str = r"[0-9a-f]{64}"
PLACEHOLDER_COMMIT: str = "0" * 40
EVIDENCE_SCRIPT: Path = Path(__file__).resolve().parents[1] / "evidence.sh"
EVIDENCE_SCRIPT_PREFIX: str = "evidence: "
DEFAULT_CLONE_DIRECTORY: str = ".evidence/clone"
CLONE_ENVIRONMENT: str = "EVIDENCE_DIR"
ROOT_ENVIRONMENT: str = "SQLQUACK_ROOT"
UNPINNED_READ_ENVIRONMENT: str = "EVIDENCE"
EVIDENCE_MAIN_BRANCH: str = "refs/remotes/origin/main"
APPEND_ONLY_FOLDERS: frozenset[str] = frozenset({"receipts", "repeats", "results"})
APPEND_ONLY_NAMES: frozenset[str] = frozenset({"withdrawn.jsonl", "inputs.jsonl"})
ADDED_STATUS: str = "A"
DELETED_STATUS: str = "D"
MODIFIED_STATUS: str = "M"
SHORT_COMMIT: int = 7
SHA256SUM_ESCAPES: tuple[tuple[bytes, bytes], ...] = (
    (b"\\", b"\\\\"),
    (b"\n", b"\\n"),
    (b"\r", b"\\r"),
)
SHA256SUM_ESCAPE_FLAG: bytes = b"\\"
