"""Checking one added referee outcome or function catalogue."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath

from scripts.evidence_guard._helpers.git import blob
from scripts.evidence_guard._helpers.metadata import valid_error
from scripts.evidence_guard.constants import (
    ACCEPTED_OUTCOME,
    EVIDENCE_FORMAT,
    HASH_LENGTH,
    HEX_DIGITS,
    METADATA_SUFFIX,
    RECORDER_PREFIX,
    REFEREE_CATALOGUE_KEYS,
    REFEREE_FUNCTIONS_PREFIX,
    REFEREE_RESULT_KEYS,
    REFEREE_RESULTS_PREFIX,
    REJECTED_OUTCOME,
)


def added_referee_problems(*, repository: Path, head: str, path: str) -> list[str]:
    """Why the added referee file `path` is not a valid new outcome or catalogue, if it is not."""
    name: PurePosixPath = PurePosixPath(path)
    if len(name.stem) != HASH_LENGTH or not set(name.stem) <= HEX_DIGITS:
        return [f"{path}: referee files are named by a lower-case SHA-256 hash"]
    if name.suffix != METADATA_SUFFIX:
        return [f"{path}: only {METADATA_SUFFIX} referee files are allowed"]
    try:
        document: object = json.loads(blob(repository=repository, head=head, path=path) or b"")
    except json.JSONDecodeError:
        return [f"{path}: not JSON"]
    if not isinstance(document, dict):
        return [f"{path}: not a JSON object"]
    if path.startswith(REFEREE_RESULTS_PREFIX):
        return _outcome_problems(path=path, stem=name.stem, document=document)
    if path.startswith(REFEREE_FUNCTIONS_PREFIX):
        return _catalogue_problems(path=path, stem=name.stem, document=document)
    return [f"{path}: referee evidence holds only results/ and functions/"]


def _outcome_problems(*, path: str, stem: str, document: dict[str, object]) -> list[str]:
    if set(document) != REFEREE_RESULT_KEYS:
        return [f"{path}: keys differ from the recorder's referee format"]
    problems: list[str] = _common_problems(path=path, document=document)
    if document["case_hash"] != stem:
        problems.append(f"{path}: wrong case hash")
    error: object = document["error"]
    if document["outcome"] == ACCEPTED_OUTCOME:
        if error is not None:
            problems.append(f"{path}: an accepted outcome keeps no error")
    elif document["outcome"] != REJECTED_OUTCOME or not valid_error(error):
        problems.append(f"{path}: a rejected outcome records its error and nothing else")
    elif document["columns"] is not None:
        problems.append(f"{path}: a rejected outcome keeps no columns")
    describe_error: object = document["describe_error"]
    if describe_error is not None and not valid_error(describe_error):
        problems.append(f"{path}: a describe error records errno, sqlstate and a message")
    return problems


def _catalogue_problems(*, path: str, stem: str, document: dict[str, object]) -> list[str]:
    if set(document) != REFEREE_CATALOGUE_KEYS:
        return [f"{path}: keys differ from the recorder's catalogue format"]
    problems: list[str] = _common_problems(path=path, document=document)
    digest: str = hashlib.sha256(
        json.dumps(document["functions"], sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if document["catalogue_sha256"] != stem or digest != stem:
        problems.append(f"{path}: the catalogue hash does not match its functions")
    return problems


def _common_problems(*, path: str, document: dict[str, object]) -> list[str]:
    problems: list[str] = []
    if document["format"] != EVIDENCE_FORMAT:
        problems.append(f"{path}: wrong format")
    if not str(document["recorder"]).startswith(RECORDER_PREFIX):
        problems.append(f"{path}: not written by sqlquack-record")
    return problems
