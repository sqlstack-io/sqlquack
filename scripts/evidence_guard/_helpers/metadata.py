"""Checking one added result."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath

from scripts.evidence_guard._helpers.git import blob
from scripts.evidence_guard.constants import (
    EVIDENCE_FORMAT,
    HASH_LENGTH,
    METADATA_KEYS,
    RECORDER_PREFIX,
    RESULTS_DIR,
)


def added_result_problems(repository: Path, head: str, path: str) -> list[str]:
    """Why the added file `path` is not a valid new result, if it is not."""
    name: PurePosixPath = PurePosixPath(path)
    stem: str = name.stem
    if len(stem) != HASH_LENGTH or any(character not in "0123456789abcdef" for character in stem):
        return [f"{path}: result files are named by a lower-case SHA-256 case hash"]
    if name.suffix == ".arrow":
        if blob(repository, head, f"{RESULTS_DIR}/{stem}.json") is None:
            return [f"{path}: an Arrow file needs its metadata file"]
        return []
    if name.suffix != ".json":
        return [f"{path}: only .json and .arrow result files are allowed"]
    return _metadata_problems(repository, head, path, stem)


def _metadata_problems(repository: Path, head: str, path: str, stem: str) -> list[str]:
    content: bytes | None = blob(repository, head, path)
    try:
        metadata: object = json.loads(content or b"")
    except json.JSONDecodeError:
        return [f"{path}: metadata is not JSON"]
    if not isinstance(metadata, dict) or set(metadata) != METADATA_KEYS:
        return [f"{path}: metadata keys differ from the recorder's format"]
    problems: list[str] = []
    if metadata["format"] != EVIDENCE_FORMAT or metadata["case_hash"] != stem:
        problems.append(f"{path}: wrong format or case hash")
    if not str(metadata["recorder"]).startswith(RECORDER_PREFIX):
        problems.append(f"{path}: not written by sqlquack-record")
    arrow: bytes | None = blob(repository, head, f"{RESULTS_DIR}/{stem}.arrow")
    if metadata["outcome"] == "rows":
        if arrow is None or metadata["arrow_sha256"] != hashlib.sha256(arrow).hexdigest():
            problems.append(f"{path}: the Arrow file is missing or does not match arrow_sha256")
    elif metadata["outcome"] != "error" or arrow is not None or metadata["error"] is None:
        problems.append(f"{path}: an error outcome needs an error and no Arrow file")
    return problems
