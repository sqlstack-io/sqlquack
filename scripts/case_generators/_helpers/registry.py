"""Every generator, and the files they write."""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from scripts.case_generators._helpers.aggregates import (
    hash_files,
    numeric_function_files,
    percentile_files,
)
from scripts.case_generators._helpers.casts import (
    cast_files,
    conversion_function_files,
    text_search_files,
)
from scripts.case_generators._helpers.json_paths import parse_files, path_files
from scripts.case_generators._helpers.json_texts import (
    composed_files,
    double_files,
    json_temporal_files,
    key_order_files,
    lenience_files,
    number_files,
)
from scripts.case_generators._helpers.numeric import (
    arithmetic_files,
    comparison_files,
    null_position_files,
)
from scripts.case_generators._helpers.objects import object_files, variant_function_files
from scripts.case_generators._helpers.temporal_texts import temporal_text_files, zone_files
from scripts.case_generators._helpers.toml_text import file_text
from scripts.case_generators.models import CaseFile
from scripts.family_cases.main.family_files import family_files
from scripts.hazard_cases.main.hazard_files import hazard_files


def generators() -> tuple[Callable[[], list[CaseFile]], ...]:
    """The generators, in the order their files are written."""
    return (
        path_files,
        parse_files,
        lenience_files,
        number_files,
        arithmetic_files,
        comparison_files,
        cast_files,
        zone_files,
        temporal_text_files,
        json_temporal_files,
        null_position_files,
        percentile_files,
        numeric_function_files,
        object_files,
        variant_function_files,
        conversion_function_files,
        text_search_files,
        hash_files,
        key_order_files,
        double_files,
        composed_files,
        hazard_files,
        family_files,
    )


def generated_texts() -> dict[str, str]:
    """Every generated file's path, relative to the generated directory, and its text."""
    texts: dict[str, str] = {}
    for generator in generators():
        for case_file in generator():
            folder: str = case_file.generator.split("@", 1)[0]
            texts[f"{folder}/{case_file.name}.toml"] = file_text(case_file)
    return texts


def stale_files(*, directory: Path, texts: dict[str, str]) -> list[str]:
    """Generated files that differ from `texts`, are missing, or no generator writes."""
    existing: set[str] = {
        path.relative_to(directory).as_posix() for path in directory.rglob("*.toml")
    }
    return sorted(
        name
        for name in set(texts) | existing
        if name not in texts
        or name not in existing
        or (directory / name).read_text() != texts[name]
    )


def write_files(*, directory: Path, texts: dict[str, str]) -> None:
    """Writes `texts` below `directory`, removing generated files no generator writes."""
    for path in directory.rglob("*.toml"):
        if path.relative_to(directory).as_posix() not in texts:
            path.unlink()
    for name, text in texts.items():
        path: Path = directory / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
