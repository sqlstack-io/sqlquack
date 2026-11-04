"""The hashes receipts are bound by."""

from __future__ import annotations

import hashlib
import json


def digest(data: bytes) -> str:
    """The SHA-256 of `data` in hex."""
    return hashlib.sha256(data).hexdigest()


def canonical_json(value: object) -> bytes:
    """The bytes every session and setup hash is taken over."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
