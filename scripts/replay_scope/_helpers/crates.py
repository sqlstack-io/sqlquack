"""The workspace crates a package is built from, read from the Cargo manifests alone."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

from scripts.replay_scope.constants import BUILD_TABLES, DIRECTORY_SUFFIX, PATH_KEY


def crate_dirs(*, root: Path, packages: tuple[str, ...]) -> frozenset[str]:
    """The directories, relative to `root` and ending in `/`, of `packages` and their path deps."""
    paths: dict[str, str] = workspace_paths(root=root)
    found: set[str] = set()
    pending: list[str] = list(packages)
    while pending:
        name: str = pending.pop()
        if name in found or name not in paths:
            continue
        found.add(name)
        pending.extend(manifest_dependencies(path=root / paths[name] / "Cargo.toml"))
    return frozenset(paths[name].rstrip(DIRECTORY_SUFFIX) + DIRECTORY_SUFFIX for name in found)


def workspace_paths(*, root: Path) -> dict[str, str]:
    """Each workspace dependency declared by path, and its directory."""
    manifest: dict[str, Any] = tomllib.loads((root / "Cargo.toml").read_text())
    declared: dict[str, Any] = manifest["workspace"].get("dependencies", {})
    return {
        name: entry[PATH_KEY]
        for name, entry in declared.items()
        if isinstance(entry, dict) and PATH_KEY in entry
    }


def manifest_dependencies(*, path: Path) -> list[str]:
    """The names a manifest's build dependency tables list, its target tables' included."""
    manifest: dict[str, Any] = tomllib.loads(path.read_text())
    tables: list[dict[str, Any]] = [manifest, *manifest.get("target", {}).values()]
    names: list[str] = []
    for table in tables:
        names.extend(table_dependencies(table=table))
    return names


def table_dependencies(*, table: dict[str, Any]) -> list[str]:
    """The names one manifest table's build dependency sections list."""
    names: list[str] = []
    for section in BUILD_TABLES:
        names.extend(table.get(section, {}))
    return names


def under(*, path: str, prefixes: tuple[str, ...] | frozenset[str]) -> bool:
    """Whether `path` is one of `prefixes` or lies in one ending in `/`."""
    return any(
        path == prefix or (prefix.endswith(DIRECTORY_SUFFIX) and path.startswith(prefix))
        for prefix in prefixes
    )
