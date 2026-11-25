"""Running one shown example in a copy of its input files and finding where the page differs."""

from __future__ import annotations

import ast
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from scripts.website_examples._helpers.comparison import difference, shows, without_progress
from scripts.website_examples.constants import (
    COMMAND_TIMEOUT_SECONDS,
    EXAMPLES_DIR,
    FILE_KEY,
    NO_INPUTS,
    OUTPUT_KEY,
    PREAMBLE_KEY,
    PROMPT,
    PYTHON_KIND,
    RESULT_COMMENT,
    SKIPPED_COMMANDS,
)
from scripts.website_examples.models import ShownBlock
from scripts.website_examples.types import Place


def block_problems(*, block: ShownBlock, root: Path, sqlquack: Path) -> list[str]:
    """Where `block` differs from its example run with `sqlquack`, or why it cannot be checked."""
    inputs: Path = root / EXAMPLES_DIR / block.example
    where: str = f"{block.page}:{block.line}: example {block.example}"
    if block.example != NO_INPUTS and not inputs.is_dir():
        return [f"{where}: {EXAMPLES_DIR}/{block.example} does not exist"]
    with tempfile.TemporaryDirectory() as directory:
        work: Path = Path(directory) / "example"
        if block.example == NO_INPUTS:
            work.mkdir()
        else:
            shutil.copytree(inputs, work)
        place: Place = (work, _environment(sqlquack=sqlquack, directory=Path(directory)))
        if block.spec.startswith(FILE_KEY):
            name: str = block.spec.removeprefix(FILE_KEY)
            return _file_problems(text=block.text, file=(work, name), where=where)
        if block.spec == PYTHON_KIND:
            return _python_problems(text=block.text, place=place, where=where)
        if block.spec.startswith(OUTPUT_KEY):
            command: str = block.spec.removeprefix(OUTPUT_KEY)
            actual: str = _run(command=command, place=place, merged=False)
            return _output_problems(shown=block.text, actual=actual, where=f"{where}: `{command}`")
        if block.spec and not block.spec.startswith(PREAMBLE_KEY):
            return [f"{where}: unknown example kind {block.spec!r}"]
        return _transcript_problems(block=block, place=place, where=where)


def _environment(*, sqlquack: Path, directory: Path) -> dict[str, str]:
    """The environment examples run in: the built `sqlquack` first on PATH and a scratch home."""
    home: Path = directory / "home"
    home.mkdir()
    path: str = os.pathsep.join((str(sqlquack.resolve().parent), os.environ.get("PATH", "")))
    return {**os.environ, "PATH": path, "HOME": str(home), "NO_COLOR": "1", "TERM": "dumb"}


def _run(*, command: str, place: Place, merged: bool) -> str:
    """What `command` prints in the shell: standard output, with standard error when `merged`."""
    work, environment = place
    finished: subprocess.CompletedProcess[str] = subprocess.run(
        ["sh", "-c", command],
        cwd=work,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT if merged else subprocess.DEVNULL,
        text=True,
        timeout=COMMAND_TIMEOUT_SECONDS,
        check=False,
    )
    return finished.stdout.removesuffix("\n")


def _unlabelled(*, text: str, name: str) -> str:
    """`text` without a first-line comment that labels it as the file `name`."""
    first, _, rest = text.partition("\n")
    labels: tuple[str, ...] = (f"-- {name}", f"# {name}")
    return rest if first.startswith(labels) else text


def _file_problems(*, text: str, file: tuple[Path, str], where: str) -> list[str]:
    """Where the shown text differs from the example's file."""
    work, name = file
    if not (work / name).is_file():
        return [f"{where}: the example has no file {name}"]
    actual: str = (work / name).read_text(encoding="utf-8").removesuffix("\n")
    shown: str = _unlabelled(text=text, name=name)
    if shown == actual:
        return []
    return [f"{where}: {name} differs from the page\n{difference(shown=shown, actual=actual)}"]


def _output_problems(*, shown: str, actual: str, where: str) -> list[str]:
    """Where the shown output differs from what the command printed."""
    printed: str = without_progress(shown=shown, actual=actual)
    if shows(shown=shown, actual=printed):
        return []
    return [f"{where} prints something else\n{difference(shown=shown, actual=printed)}"]


def _transcript_problems(*, block: ShownBlock, place: Place, where: str) -> list[str]:
    """Where a shown session of `$` commands differs from running them in order."""
    lines: list[str] = block.text.split("\n")
    starts: list[int] = [index for index, line in enumerate(lines) if line.startswith(PROMPT)]
    if not starts:
        return [f"{where}: the block shows no `{PROMPT.strip()}` command"]
    problems: list[str] = []
    preamble: str = "\n".join(lines[: starts[0]]).strip("\n")
    if block.spec.startswith(PREAMBLE_KEY):
        shown_file: tuple[Path, str] = (place[0], block.spec.removeprefix(PREAMBLE_KEY))
        problems += _file_problems(text=preamble, file=shown_file, where=where)
    elif preamble:
        problems.append(f"{where}: lines before the first command need a preamble= file")
    for start, end in zip(starts, [*starts[1:], len(lines)], strict=True):
        command: str = lines[start].removeprefix(PROMPT)
        shown: str = "\n".join(lines[start + 1 : end]).strip("\n")
        if command in SKIPPED_COMMANDS:
            continue
        actual: str = _run(command=command, place=place, merged=True)
        problems += _output_problems(shown=shown, actual=actual, where=f"{where}: `{command}`")
    return problems


def _python_problems(*, text: str, place: Place, where: str) -> list[str]:
    """Where a value a Python block shows in a `# value` comment differs from the one it gives."""
    work, environment = place
    script: list[str] = ["_shown = []"]
    expected: dict[int, object] = {}
    for number, line in enumerate(text.split("\n"), start=1):
        found: re.Match[str] | None = RESULT_COMMENT.match(line)
        value: object = _literal(found["value"]) if found else None
        if found is None or value is None:
            script.append(line)
            continue
        expected[number] = value
        script.append(f"_shown.append(({number}, repr({found['expression']})))")
    script.append("import json\nprint(json.dumps(_shown))")
    finished: subprocess.CompletedProcess[str] = subprocess.run(
        [sys.executable, "-c", "\n".join(script)],
        cwd=work,
        env=environment,
        capture_output=True,
        text=True,
        timeout=COMMAND_TIMEOUT_SECONDS,
        check=False,
    )
    if finished.returncode != 0:
        return [f"{where}: the Python block fails\n{finished.stderr.strip()}"]
    given: dict[int, object] = {
        line: ast.literal_eval(shown) for line, shown in json.loads(finished.stdout)
    }
    return [
        f"{where}: line {line} shows {value!r}, but gives {given[line]!r}"
        for line, value in expected.items()
        if given[line] != value
    ]


def _literal(text: str) -> object:
    """The Python literal a result comment shows, or None when it shows none."""
    try:
        return ast.literal_eval(text)
    except (SyntaxError, ValueError):
        return None
