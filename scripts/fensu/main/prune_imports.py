"""Entry point: drop the imports a split added that no build uses; gate test-only ones on tests."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

from scripts.fensu.constants import (
    COMPILER_MESSAGE,
    REMOVE,
    TEST_PATH_PARTS,
    TESTS_FILE,
    UNUSED_IMPORTS_LINT,
    USE_PREFIXES,
)
from scripts.fensu.exceptions import TransformError


def run_prune_imports(argv: list[str] | None = None) -> int:
    """Check the library with and without tests and rewrite the `use` lines neither build needs."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", default="sqlquack-syntax")
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    arguments: argparse.Namespace = parser.parse_args(argv)
    print("prune-imports: checking the library with and without its tests", file=sys.stderr)
    library: set[tuple[str, int]] = _unused(
        repository=arguments.repository, package=arguments.package, tests=False
    )
    tested: set[tuple[str, int]] = _unused(
        repository=arguments.repository, package=arguments.package, tests=True
    )
    edits: dict[str, dict[int, str]] = {}
    for file, line in tested:
        if _test_only(file) or (file, line) in library:
            edits.setdefault(file, {})[line] = "remove"
    for file, line in library - tested:
        edits.setdefault(file, {})[line] = "gate"
    removed: int = 0
    gated: int = 0
    for file, lines in sorted(edits.items()):
        path: Path = arguments.repository / file
        text_lines: list[str] = path.read_text().split("\n")
        for line, action in sorted(lines.items(), reverse=True):
            current: str = text_lines[line - 1]
            if not current.lstrip().startswith(USE_PREFIXES):
                raise TransformError(f"{file}:{line} is not a one-line use declaration")
            if action == REMOVE:
                del text_lines[line - 1]
                removed += 1
            else:
                indent: str = current[: len(current) - len(current.lstrip())]
                text_lines.insert(line - 1, f"{indent}#[cfg(test)]")
                gated += 1
        path.write_text("\n".join(text_lines))
    print(f"prune-imports: done, {removed} removed, {gated} gated on tests", file=sys.stderr)
    return 0


def _test_only(file: str) -> bool:
    parts: list[str] = file.split("/")
    return parts[-1] == TESTS_FILE or any(part in TEST_PATH_PARTS for part in parts[:-1])


def _unused(*, repository: Path, package: str, tests: bool) -> set[tuple[str, int]]:
    command: list[str] = [
        "cargo",
        "check",
        "--locked",
        "-p",
        package,
        "--lib",
        "--message-format=json",
    ]
    if tests:
        command.extend(["--profile", "test"])
    output: str = subprocess.run(
        command, cwd=repository, capture_output=True, text=True, check=False
    ).stdout
    unused: set[tuple[str, int]] = set()
    for line in output.splitlines():
        message: dict = json.loads(line)
        if message.get("reason") != COMPILER_MESSAGE:
            continue
        diagnostic: dict = message["message"]
        code: dict | None = diagnostic.get("code")
        if code is None or code.get("code") != UNUSED_IMPORTS_LINT:
            continue
        for span in diagnostic["spans"]:
            unused.add((span["file_name"], span["line_start"]))
    return unused
