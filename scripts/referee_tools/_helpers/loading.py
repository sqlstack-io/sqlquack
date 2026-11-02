"""Reading every referee file and environment below `referee/`."""

from __future__ import annotations

import hashlib
import json
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from scripts.referee_tools.constants import (
    ENVIRONMENTS_DIRECTORY,
    JSON_SUFFIX,
    REFEREE_DIRECTORY,
    REFEREE_FORMAT,
    SOURCE_DIALECT,
)
from scripts.referee_tools.models import EnvironmentFile, RefereeCase


def referee_cases(*, repository: Path, patterns: tuple[str, ...]) -> list[RefereeCase]:
    """Every statement of the referee files matching any of `patterns`, in path order."""
    root: Path = repository / REFEREE_DIRECTORY
    environments: dict[str, EnvironmentFile] = {
        path.stem: _environment(path)
        for path in sorted((root / ENVIRONMENTS_DIRECTORY).glob(f"*{JSON_SUFFIX}"))
    }
    paths: set[Path] = set()
    for pattern in patterns:
        paths.update(path for path in root.glob(pattern) if path.is_file())
    cases: list[RefereeCase] = []
    for path in sorted(paths):
        if path.parent.name != ENVIRONMENTS_DIRECTORY:
            cases.extend(_file_cases(path=path, root=root, environments=environments))
    return cases


def _environment(path: Path) -> EnvironmentFile:
    document: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    parameters: dict[str, bool | int | str] = dict(document["session_parameters"])
    setup: tuple[str, ...] = tuple(document["setup_sql"])
    return EnvironmentFile(
        name=path.stem,
        session_parameters=parameters,
        setup_sql=setup,
        environment_sha256=environment_sha256(session_parameters=parameters, setup_sql=setup),
    )


def _file_cases(
    *, path: Path, root: Path, environments: dict[str, EnvironmentFile]
) -> list[RefereeCase]:
    document: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    environment: EnvironmentFile = environments[document["environment"]]
    syntax_only: bool = bool(document["syntax_only"])
    reference: str = path.relative_to(root).as_posix()
    return [
        RefereeCase(
            case_hash=case_sha256(
                source_dialect=SOURCE_DIALECT,
                environment=environment.environment_sha256,
                statement=entry["sql"],
                syntax_only=syntax_only,
            ),
            case_reference=f"{reference}#{entry['name']}",
            environment=environment,
            statement=entry["sql"],
            syntax_only=syntax_only,
            kind=entry["kind"],
            mutation=entry.get("mutation"),
            base=entry.get("base"),
        )
        for entry in document["statements"]
    ]


def environment_sha256(
    *, session_parameters: Mapping[str, bool | int | str], setup_sql: Sequence[str]
) -> str:
    """SHA-256 of an environment's canonical JSON."""
    return _canonical_sha256(
        {
            "format": REFEREE_FORMAT,
            "session_parameters": dict(session_parameters),
            "setup_sql": list(setup_sql),
        }
    )


def case_sha256(*, source_dialect: str, environment: str, statement: str, syntax_only: bool) -> str:
    """SHA-256 of a case's canonical JSON, which names its environment by hash."""
    return _canonical_sha256(
        {
            "format": REFEREE_FORMAT,
            "source_dialect": source_dialect,
            "environment_sha256": environment,
            "statement": statement,
            "syntax_only": syntax_only,
        }
    )


def _canonical_sha256(document: object) -> str:
    text: str = json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()
