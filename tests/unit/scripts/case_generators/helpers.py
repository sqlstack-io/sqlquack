"""Tampering with a generated directory."""

from __future__ import annotations

from pathlib import Path

from scripts.case_generators.constants import GENERATED_DIRECTORY


def untouched(_: Path) -> None:
    """Leaves the generated files as written."""


def edited(repository: Path) -> None:
    """Hand-edits one generated file."""
    path: Path = repository / GENERATED_DIRECTORY / "time_zones" / "convert_timezone.toml"
    path.write_text(path.read_text().replace("'UTC'", "'Etc/UTC'", 1))


def stray(repository: Path) -> None:
    """Adds a file no generator writes."""
    (repository / GENERATED_DIRECTORY / "time_zones" / "extra.toml").write_text("")


def removed(repository: Path) -> None:
    """Deletes one generated file."""
    (repository / GENERATED_DIRECTORY / "time_zones" / "convert_timezone.toml").unlink()
