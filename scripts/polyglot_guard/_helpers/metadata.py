"""Normal-dependency reachability from Cargo's resolved graph."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from scripts.polyglot_guard.types import Metadata, NodeDependency, ResolveNode


def cargo_metadata(*, repository: Path) -> Metadata:
    """Cargo's resolved dependency graph for the workspace, from the lockfile."""
    output: str = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    metadata: Metadata = json.loads(output)
    return metadata


def dependents_of(*, metadata: Metadata, package: str) -> set[str]:
    """Workspace members that reach `package` through normal dependencies, not dev or build."""
    names: dict[str, str] = {entry["id"]: entry["name"] for entry in metadata["packages"]}
    edges: dict[str, list[str]] = {
        node["id"]: _normal_dependencies(node=node) for node in metadata["resolve"]["nodes"]
    }
    reaching: set[str] = set()
    for member in metadata["workspace_members"]:
        if _reaches(start=member, package=package, edges=edges, names=names):
            reaching.add(names[member])
    return reaching


def _normal_dependencies(*, node: ResolveNode) -> list[str]:
    return [dependency["pkg"] for dependency in node["deps"] if _is_normal(dependency=dependency)]


def _is_normal(*, dependency: NodeDependency) -> bool:
    return any(kind["kind"] is None for kind in dependency["dep_kinds"])


def _reaches(
    *, start: str, package: str, edges: dict[str, list[str]], names: dict[str, str]
) -> bool:
    seen: set[str] = {start}
    pending: list[str] = [start]
    while pending:
        for dependency in edges.get(pending.pop(), []):
            if names[dependency] == package:
                return True
            if dependency not in seen:
                seen.add(dependency)
                pending.append(dependency)
    return False
