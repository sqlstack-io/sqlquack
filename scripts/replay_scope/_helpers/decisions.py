"""What a change replays and checks, the replay cache's scope, and the declared paths' check."""

from __future__ import annotations

import hashlib
from pathlib import Path

from scripts.replay_scope._helpers.crates import crate_dirs, manifest_dependencies, under
from scripts.replay_scope._helpers.git import changed, tracked
from scripts.replay_scope.constants import (
    BINARY_PACKAGE,
    DIRECTORY_SUFFIX,
    DOCUMENTATION_PATHS,
    FULL_REPLAY_PATHS,
    MARKDOWN_SUFFIX,
    REPLAY_INPUT_PATHS,
    SITE_MANIFEST,
    SITE_PATHS,
)
from scripts.replay_scope.types import Scope


def replay_scope(*, paths: list[str], binary_dirs: frozenset[str]) -> Scope:
    """`full` when a changed path forces it, `changed` when one feeds the replay, else `none`."""
    if any(under(path=path, prefixes=FULL_REPLAY_PATHS) for path in paths):
        return "full"
    if any(feeds_replay(path=path, binary_dirs=binary_dirs) for path in paths):
        return "changed"
    return "none"


def feeds_replay(*, path: str, binary_dirs: frozenset[str]) -> bool:
    """Whether `path` is in a crate the binary is built from or among the replay's inputs."""
    return under(path=path, prefixes=binary_dirs) or under(path=path, prefixes=REPLAY_INPUT_PATHS)


def needs_code_checks(*, paths: list[str]) -> bool:
    """Whether a changed path lies outside the documentation and the top-level Markdown files."""
    return any(not documentation(path=path) for path in paths)


def documentation(*, path: str) -> bool:
    """Whether `path` is documentation: under a documentation path, or top-level Markdown."""
    top_level_markdown: bool = path.endswith(MARKDOWN_SUFFIX) and DIRECTORY_SUFFIX not in path
    return top_level_markdown or under(path=path, prefixes=DOCUMENTATION_PATHS)


def needs_site(*, paths: list[str], site_dirs: frozenset[str]) -> bool:
    """Whether a changed path is in the site or a crate the playground engine is built from."""
    return any(
        under(path=path, prefixes=SITE_PATHS) or under(path=path, prefixes=site_dirs)
        for path in paths
    )


def change_plan(*, root: Path, base: str) -> dict[str, str]:
    """The replay scope and whether the code and site checks run, as GitHub step outputs."""
    paths: list[str] = changed(root=root, base=base)
    binary_dirs: frozenset[str] = crate_dirs(root=root, packages=(BINARY_PACKAGE,))
    engine: tuple[str, ...] = tuple(manifest_dependencies(path=root / SITE_MANIFEST))
    site_dirs: frozenset[str] = crate_dirs(root=root, packages=engine)
    return {
        "replay": replay_scope(paths=paths, binary_dirs=binary_dirs),
        "code": str(needs_code_checks(paths=paths)).lower(),
        "site": str(needs_site(paths=paths, site_dirs=site_dirs)).lower(),
    }


def scope_digest(*, root: Path) -> str:
    """The SHA-256 of every tracked file under the full-replay paths, by path and content."""
    parts: list[bytes] = []
    for path in tracked(root=root, prefixes=FULL_REPLAY_PATHS):
        content: bytes = (root / path).read_bytes()
        parts.extend((f"{path}\0{len(content)}\0".encode(), content))
    return hashlib.sha256(b"".join(parts)).hexdigest()


def unmatched(*, root: Path) -> list[str]:
    """The full-replay and replay-input paths no tracked file matches."""
    files: list[str] = tracked(root=root, prefixes=())
    declared: tuple[str, ...] = (*FULL_REPLAY_PATHS, *REPLAY_INPUT_PATHS)
    return [prefix for prefix in declared if not matches_any(prefix=prefix, files=files)]


def matches_any(*, prefix: str, files: list[str]) -> bool:
    """Whether any of `files` is `prefix` or lies under it."""
    return any(under(path=path, prefixes=(prefix,)) for path in files)
