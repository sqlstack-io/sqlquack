"""Probes read from the sources file, their argument types completed from the samples."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.guard_measurement.constants import PARQUET_SUFFIX, TYPES_SUFFIX, UNKNOWN_TYPE
from scripts.guard_measurement.models import Probe


def read_probes(*, path: Path) -> list[Probe]:
    """Every probe in the sources file's `probes` list."""
    loaded: dict[str, list[dict[str, object]]] = json.loads(path.read_text(encoding="utf-8"))
    return [_probe(entry=entry) for entry in loaded.get("probes", [])]


def sample_file(*, samples: Path, relation: str) -> Path:
    """The Parquet sample of `relation`."""
    return samples / f"{relation}{PARQUET_SUFFIX}"


def sample_types(*, samples: Path, relation: str) -> dict[str, str]:
    """The Snowflake column types the sampler recorded for `relation`, upper-case names."""
    path: Path = samples / f"{relation}{TYPES_SUFFIX}"
    if not path.is_file():
        return {}
    recorded: dict[str, str] = json.loads(path.read_text(encoding="utf-8"))
    return {name.upper(): data_type for name, data_type in recorded.items()}


def typed_arguments(
    *, probe: Probe, recorded: dict[str, str]
) -> tuple[tuple[str, str], ...] | None:
    """The probe's arguments with UNKNOWN types taken from `recorded`; None if one stays unknown."""
    typed: list[tuple[str, str]] = []
    for name, data_type in probe.arguments:
        resolved: str = (
            recorded.get(name.upper(), data_type) if data_type == UNKNOWN_TYPE else data_type
        )
        if resolved == UNKNOWN_TYPE:
            return None
        typed.append((name, resolved))
    return tuple(typed)


def _probe(*, entry: dict[str, object]) -> Probe:
    """One probe entry."""
    arguments: object = entry["arguments"]
    pairs: list[list[object]] = arguments if isinstance(arguments, list) else []
    flag: object = entry.get("semi_structured")
    return Probe(
        divergence=str(entry["divergence"]),
        construct=str(entry["construct"]),
        relation=str(entry["relation"]),
        expression=str(entry["expression"]),
        arguments=tuple((str(pair[0]), str(pair[1])) for pair in pairs),
        semi_structured=flag if isinstance(flag, bool) else None,
        aggregate=bool(entry.get("aggregate", False)),
    )
