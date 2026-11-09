"""Reading SQLQuack's recorded evidence and writing it in the evidence repository's layout."""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any

from scripts.evidence_publish._helpers.receipts import (
    line_of,
    publish_lines,
    published_case,
    published_case_id,
)
from scripts.evidence_publish.constants import (
    CASE_ID_MIN_PARTS,
    CATALOGUE_DIRECTORY,
    COPIED,
    DIALECT_SYNTAX_ROOT,
    INPUT_KEYS,
    INPUTS_FILE,
    RECEIPT_HASH_KEY,
    RECEIPTS_DIRECTORIES,
    RECEIPTS_DIRECTORY,
    REPORT_INPUTS,
    REPORT_SUFFIX,
    REPORT_WITHDRAWN,
    SNOWFLAKE,
    SQL_HASH_KEY,
    SQL_KEY,
    SYNTAX_COPIED,
    SYNTAX_FOLDER,
    SYNTAX_ROOT,
    UNCATALOGUED_FILE,
    WITHDRAWN_FILE,
)


def syntax_root(*, repository: Path, dialect: str) -> Path:
    """The dialect's syntax-coverage root in SQLQuack today."""
    relative: str = (
        SYNTAX_ROOT if dialect == SNOWFLAKE else DIALECT_SYNTAX_ROOT.format(dialect=dialect)
    )
    return repository / relative


def write_lines(*, path: Path, lines: list[str]) -> None:
    """Write `lines` to `path`, one per line, creating its folder."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")


def copy_cases(*, root: Path, target: Path) -> None:
    """Copy a root's case files with their capture methods under their published names."""
    for source, destination in SYNTAX_COPIED:
        for path in sorted((root / source).glob("*.jsonl")):
            lines: list[str] = path.read_text(encoding="utf-8").splitlines()
            published: list[str] = [published_case(line) for line in lines]
            write_lines(path=target / destination / path.name, lines=published)


def receipt_lines(directory: Path) -> list[str]:
    """Every receipt line of a directory, in file-name order, then line order."""
    lines: list[str] = []
    for path in sorted(directory.glob("*.jsonl")):
        lines.extend(line for line in path.read_text(encoding="utf-8").splitlines() if line)
    return lines


def family_file(*, case_id: str, known: frozenset[str]) -> str:
    """The family file a receipt goes in: its case id's family, or the uncatalogued file."""
    parts: list[str] = case_id.split(".")
    catalogued: bool = len(parts) >= CASE_ID_MIN_PARTS and parts[1] in known
    return f"{parts[1]}.jsonl" if catalogued else UNCATALOGUED_FILE


def write_families(*, directory: Path, lines: list[str], known: frozenset[str]) -> None:
    """Write published receipt lines into one file per family, sorted by case, time and order."""
    grouped: dict[str, list[tuple[str, str, int, str]]] = {}
    for index, line in enumerate(lines):
        record: dict[str, Any] = json.loads(line)
        name: str = family_file(case_id=record["case_id"], known=known)
        grouped.setdefault(name, []).append((record["case_id"], record["recorded_at"], index, line))
    for name, entries in grouped.items():
        write_lines(path=directory / name, lines=[entry[3] for entry in sorted(entries)])


def write_receipts(*, root: Path, target: Path) -> dict[str, str]:
    """Publish a root's receipts and repeats by family; return the old to new line hashes."""
    known: frozenset[str] = frozenset(
        path.stem for path in (root / CATALOGUE_DIRECTORY).glob("*.toml")
    )
    private: dict[str, list[str]] = {
        name: receipt_lines(root / name) for name in RECEIPTS_DIRECTORIES
    }
    combined: list[str] = []
    for name in RECEIPTS_DIRECTORIES:
        combined.extend(private[name])
    published, mapping = publish_lines(combined)
    start: int = 0
    for name in RECEIPTS_DIRECTORIES:
        end: int = start + len(private[name])
        write_families(directory=target / name, lines=published[start:end], known=known)
        start = end
    return mapping


def input_line(entry: dict[str, Any]) -> str:
    """One published input: its SQL and, when it had one, its setup."""
    return line_of({key: entry[key] for key in INPUT_KEYS if key in entry})


def report_inputs(report: dict[str, Any]) -> set[str]:
    """Each input a report read, as a published line."""
    return {
        input_line(entry)
        for entry in report.get(REPORT_INPUTS) or []
        if SQL_KEY in entry and SQL_HASH_KEY in entry
    }


def withdrawn_line(*, entry: dict[str, Any], mapping: dict[str, str]) -> str:
    """A published withdrawn entry: the receipt's new line hash, its case and the reason."""
    return line_of(
        {
            RECEIPT_HASH_KEY: mapping[entry[RECEIPT_HASH_KEY]],
            "case_id": published_case_id(entry["case_id"]),
            "reason": entry["reason"],
        }
    )


def write_lists(*, root: Path, target: Path, mapping: dict[str, str]) -> None:
    """Write the withdrawn receipts and the reports' inputs beside the published receipts."""
    withdrawn: set[str] = set()
    inputs: set[str] = set()
    for path in sorted((root / RECEIPTS_DIRECTORY).glob(f"*{REPORT_SUFFIX}")):
        report: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
        entries: list[dict[str, Any]] = report.get(REPORT_WITHDRAWN) or []
        withdrawn.update(withdrawn_line(entry=entry, mapping=mapping) for entry in entries)
        inputs.update(report_inputs(report))
    if withdrawn:
        write_lines(path=target / WITHDRAWN_FILE, lines=sorted(withdrawn))
    if inputs:
        write_lines(path=target / INPUTS_FILE, lines=sorted(inputs))


def publish_syntax(*, repository: Path, dialect: str, out: Path) -> dict[str, str]:
    """Write one dialect's published syntax evidence; return its old to new receipt hashes."""
    root: Path = syntax_root(repository=repository, dialect=dialect)
    target: Path = out / SYNTAX_FOLDER / dialect
    copy_cases(root=root, target=target)
    mapping: dict[str, str] = write_receipts(root=root, target=target)
    write_lists(root=root, target=target, mapping=mapping)
    return mapping


def copy_evidence(*, repository: Path, out: Path) -> None:
    """Copy the translation and referee evidence unchanged into their published folders."""
    for source, destination in COPIED:
        path: Path = repository / source
        if path.is_dir():
            shutil.copytree(path, out / destination)
        else:
            (out / destination).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, out / destination)
