"""Synthetic cargo metadata for the polyglot guard."""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path

from tests.unit.scripts.polyglot_guard._test_types import Edge


def write_metadata(*, directory: Path, members: tuple[str, ...], edges: tuple[Edge, ...]) -> Path:
    """A cargo metadata file with `members` as workspace members and `edges` as the graph."""
    dependencies: defaultdict[str, list[dict[str, object]]] = defaultdict(list)
    names: set[str] = set(members)
    for dependent, dependency, kind in edges:
        dependencies[dependent].append({"pkg": dependency, "dep_kinds": [{"kind": kind}]})
        names.update((dependent, dependency))
    metadata: dict[str, object] = {
        "packages": [{"id": name, "name": name} for name in sorted(names)],
        "workspace_members": list(members),
        "resolve": {"nodes": [{"id": name, "deps": dependencies[name]} for name in sorted(names)]},
    }
    path: Path = directory / "metadata.json"
    path.write_text(json.dumps(metadata))
    return path
