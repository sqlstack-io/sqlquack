"""The content hash of an evidence snapshot, the same as scripts/evidence.sh computes."""

from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path

from scripts.evidence_pin.constants import SHA256SUM_ESCAPE_FLAG, SHA256SUM_ESCAPES


def content_sha256(root: Path) -> str:
    """The SHA-256 of one `<sha256>  <path>` line per regular file under `root`, sorted by path."""
    lines: list[bytes] = []
    for relative in sorted(_regular_files(root)):
        digest: str = hashlib.sha256((root / os.fsdecode(relative)).read_bytes()).hexdigest()
        lines.append(_sha256sum_line(digest=digest, name=relative))
    return hashlib.sha256(b"".join(lines)).hexdigest()


def _regular_files(root: Path) -> list[bytes]:
    """The path of every regular file under `root`, relative to it, without following links."""
    files: list[bytes] = []
    for directory, _, names in os.walk(root):
        for name in names:
            path: str = os.path.join(directory, name)
            if stat.S_ISREG(os.lstat(path).st_mode):
                files.append(os.fsencode(os.path.relpath(path, root)))
    return files


def _sha256sum_line(*, digest: str, name: bytes) -> bytes:
    """The line `sha256sum` prints for a file: names with special bytes escaped and flagged."""
    escaped: bytes = name
    for raw, replacement in SHA256SUM_ESCAPES:
        escaped = escaped.replace(raw, replacement)
    flag: bytes = SHA256SUM_ESCAPE_FLAG if escaped != name else b""
    return flag + digest.encode("ascii") + b"  " + escaped + b"\n"
