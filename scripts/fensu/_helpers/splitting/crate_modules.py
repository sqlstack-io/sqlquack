"""A crate's modules as written: their items, their imports and how a path resolves among them."""

from __future__ import annotations

from pathlib import Path

from scripts.fensu.constants import ROOT_FILES
from scripts.fensu.exceptions import TransformError
from scripts.fensu.models import Definition, Item, SourceModule


def visible(*, definition: Definition, item: Item, importer: tuple[str, ...]) -> bool:
    if item.visibility:
        return True
    return importer[: len(definition.module)] == definition.module


def parent_of(module: SourceModule) -> SourceModule:
    if module.parent is None:
        raise TransformError(f"{module.path} has no parent module")
    return module.parent


def child_file(*, module: SourceModule, name: str, item: Item) -> Path:
    for attribute in item.attributes:
        if attribute.startswith("#[path"):
            relative: str = attribute.split('"')[1]
            return module.file.parent / relative
    directory: Path = module.file.parent
    if module.file.name not in ROOT_FILES:
        directory = directory / module.file.stem
    if (directory / f"{name}.rs").exists():
        return directory / f"{name}.rs"
    return directory / name / "mod.rs"
