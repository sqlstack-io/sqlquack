"""Checking one added result."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath

from scripts.evidence_guard._helpers.git import blob
from scripts.evidence_guard.constants import (
    ARROW_SUFFIX,
    ERROR_KEYS,
    ERROR_OUTCOME,
    EVIDENCE_FORMAT,
    HASH_LENGTH,
    HEX_DIGITS,
    METADATA_KEYS,
    METADATA_SUFFIX,
    RECORDER_PREFIX,
    RESULTS_DIR,
    ROWS_OUTCOME,
)


def added_result_problems(*, repository: Path, head: str, path: str) -> list[str]:
    """Why the added file `path` is not a valid new result, if it is not."""
    name: PurePosixPath = PurePosixPath(path)
    stem: str = name.stem
    if len(stem) != HASH_LENGTH or not set(stem) <= HEX_DIGITS:
        return [f"{path}: result files are named by a lower-case SHA-256 case hash"]
    sibling: str = f"{RESULTS_DIR}/{stem}{METADATA_SUFFIX}"
    if name.suffix == ARROW_SUFFIX:
        present: bool = blob(repository=repository, head=head, path=sibling) is not None
        return [] if present else [f"{path}: an Arrow file needs its metadata file"]
    if name.suffix != METADATA_SUFFIX:
        return [f"{path}: only {METADATA_SUFFIX} and {ARROW_SUFFIX} result files are allowed"]
    return _metadata_problems(repository=repository, head=head, path=path, stem=stem)


def _metadata_problems(*, repository: Path, head: str, path: str, stem: str) -> list[str]:
    content: bytes | None = blob(repository=repository, head=head, path=path)
    try:
        metadata: object = json.loads(content or b"")
    except json.JSONDecodeError:
        return [f"{path}: metadata is not JSON"]
    if not isinstance(metadata, dict) or set(metadata) != METADATA_KEYS:
        return [f"{path}: metadata keys differ from the recorder's format"]
    problems: list[str] = []
    if metadata["format"] != EVIDENCE_FORMAT or metadata["case_hash"] != stem:
        problems.append(f"{path}: wrong format or case hash")
    query_hash: object = metadata["query_id_sha256"]
    if query_hash is not None and (
        not isinstance(query_hash, str)
        or len(query_hash) != HASH_LENGTH
        or not set(query_hash) <= HEX_DIGITS
    ):
        problems.append(f"{path}: query_id_sha256 must be a SHA-256 hex digest or null")
    error: object = metadata["error"]
    if error is not None and (not isinstance(error, dict) or set(error) != ERROR_KEYS):
        problems.append(f"{path}: an error records only errno and sqlstate")
    if not str(metadata["recorder"]).startswith(RECORDER_PREFIX):
        problems.append(f"{path}: not written by sqlquack-record")
    arrow: bytes | None = blob(
        repository=repository, head=head, path=f"{RESULTS_DIR}/{stem}{ARROW_SUFFIX}"
    )
    if metadata["outcome"] == ROWS_OUTCOME:
        if arrow is None or metadata["arrow_sha256"] != hashlib.sha256(arrow).hexdigest():
            problems.append(f"{path}: the Arrow file is missing or does not match arrow_sha256")
    elif metadata["outcome"] != ERROR_OUTCOME or arrow is not None or metadata["error"] is None:
        problems.append(f"{path}: an error outcome needs an error and no Arrow file")
    return problems
