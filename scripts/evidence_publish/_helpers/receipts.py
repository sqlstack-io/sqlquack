"""Rewriting private receipt lines into published ones, with their line hashes remapped."""

from __future__ import annotations

import copy
import hashlib
import json
from typing import Any

from scripts.evidence_publish.constants import (
    BASIS_REWRITES,
    CAPTURE_FIELD,
    CAPTURE_PARTS,
    CAPTURE_RENAMES,
    CASE_ID_PREFIX_RENAMES,
    DECISION_PREFIX,
    DECISION_SUFFIX,
    HOME_PATH,
    HOME_PLACEHOLDER,
    MEASURE_RENAMES,
    MESSAGE_FIELDS,
    PRIVATE_RECEIPT_FIELDS,
    PUBLIC_RECEIPT_FORMAT,
    QUERY_ID_FIELD,
    QUERY_ID_HASH_FIELD,
    RUN_PROJECT,
    RUN_PROJECT_PLACEHOLDER,
    SUPERSEDES_FIELD,
)
from scripts.evidence_publish.exceptions import SupersessionError


def sha256_hex(text: str) -> str:
    """The SHA-256 of `text`'s UTF-8 bytes."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def line_of(record: dict[str, Any]) -> str:
    """One receipt line, written the way every importer writes it."""
    return json.dumps(record, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def neutral_basis(text: str) -> str:
    """`text` without decision numbers or author names, naming captures by their published names."""
    text = DECISION_SUFFIX.sub("", DECISION_PREFIX.sub("", text))
    for old, new in BASIS_REWRITES.items():
        text = text.replace(old, new)
    for old, new in CAPTURE_RENAMES.items():
        text = text.replace(old, new)
    return text


def published_case(line: str) -> str:
    """A syntax case line with its capture method under its published name, otherwise unchanged."""
    for old, new in CAPTURE_RENAMES.items():
        line = line.replace(f'"capture":"{old}"', f'"capture":"{new}"')
    return line


def published_case_id(case_id: str) -> str:
    """A receipt's case id with a run-named prefix under its published name."""
    for old, new in CASE_ID_PREFIX_RENAMES.items():
        if case_id.startswith(old):
            return new + case_id.removeprefix(old)
    return case_id


def published(*, record: dict[str, Any], mapping: dict[str, str]) -> dict[str, Any]:
    """The published form of one private receipt; `mapping` gives superseded lines' new hashes."""
    out: dict[str, Any] = {
        key: copy.deepcopy(value)
        for key, value in record.items()
        if key not in PRIVATE_RECEIPT_FIELDS and key not in CAPTURE_PARTS
    }
    out["format"] = PUBLIC_RECEIPT_FORMAT
    out["case_id"] = published_case_id(out["case_id"])
    query_id: str | None = out.pop(QUERY_ID_FIELD, None)
    if query_id is not None:
        out[QUERY_ID_HASH_FIELD] = out.get(QUERY_ID_HASH_FIELD) or sha256_hex(query_id)
    out[CAPTURE_FIELD] = sha256_hex(" ".join(record[part] for part in CAPTURE_PARTS))
    out["capture_method"] = CAPTURE_RENAMES.get(out["capture_method"], out["capture_method"])
    if out.get("measure") in MEASURE_RENAMES:
        out["measure"] = MEASURE_RENAMES[out["measure"]]
    out["phase_basis"] = neutral_basis(out["phase_basis"])
    for field in MESSAGE_FIELDS:
        if isinstance(out.get(field), str):
            masked: str = RUN_PROJECT.sub(RUN_PROJECT_PLACEHOLDER, out[field])
            out[field] = HOME_PATH.sub(HOME_PLACEHOLDER, masked)
    if out.get(SUPERSEDES_FIELD):
        out[SUPERSEDES_FIELD] = mapping[out[SUPERSEDES_FIELD]]
    for setting in (out.get("account_state") or {}).values():
        if isinstance(setting.get("basis"), str):
            setting["basis"] = neutral_basis(setting["basis"])
    return out


def publish_lines(lines: list[str]) -> tuple[list[str], dict[str, str]]:
    """Every line's published line in order, and each old line hash's new hash; superseded first."""
    records: list[dict[str, Any]] = [json.loads(line) for line in lines]
    old_hashes: list[str] = [sha256_hex(line) for line in lines]
    mapping: dict[str, str] = {}
    out: list[str] = [""] * len(lines)
    pending: list[int] = list(range(len(lines)))
    while pending:
        ready: set[int] = {
            index
            for index in pending
            if not records[index].get(SUPERSEDES_FIELD)
            or records[index][SUPERSEDES_FIELD] in mapping
        }
        if not ready:
            raise SupersessionError(f"{len(pending)} receipts supersede no published line")
        for index in sorted(ready):
            text: str = line_of(published(record=records[index], mapping=mapping))
            mapping[old_hashes[index]] = sha256_hex(text)
            out[index] = text
        pending = [index for index in pending if index not in ready]
    return out, mapping
