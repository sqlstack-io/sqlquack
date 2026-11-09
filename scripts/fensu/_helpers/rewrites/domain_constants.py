"""The domain constants.rs files that named literals go to, and the imports of them."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.fensu._helpers.rewrites.literals import compared_literals, constant_name, literal_edits
from scripts.fensu._helpers.rust.comments import apply_edits
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.constants import CONSTANTS_FILE, CONSTANTS_MODULE, MOD_FILE
from scripts.fensu.exceptions import TransformError
from scripts.fensu.models import Token


def named_in_file(*, path: Path, domain: str, values: dict[str, str]) -> dict[str, str]:
    """Rewrite the compared literals of `path` to constant names; `values` plus those named."""
    text: str = path.read_text()
    literals: list[Token] = compared_literals(tokens=tokenize(text=text))
    if not literals:
        return values
    for token in literals:
        values = recorded(values=values, name=constant_name(token.text), literal=token.text)
    rewritten: str = apply_edits(text=text, edits=literal_edits(literals=literals))
    names: set[str] = {constant_name(token.text) for token in literals}
    path.write_text(with_imports(text=rewritten, domain=domain, names=names))
    return values


def recorded(*, values: dict[str, str], name: str, literal: str) -> dict[str, str]:
    """`values` with `name` naming `literal`, which it must not already name differently."""
    if values.get(name, literal) != literal:
        raise TransformError(f"{name} would name both {values[name]} and {literal}")
    return {**values, name: literal}


def with_imports(*, text: str, domain: str, names: set[str]) -> str:
    """`text` importing each of `names` from its domain's constants."""
    lines: list[str] = text.split("\n")
    wanted: list[str] = [
        f"use crate::{domain}::{CONSTANTS_MODULE}::{name};"
        for name in sorted(names)
        if f"use crate::{domain}::{CONSTANTS_MODULE}::{name};" not in lines
    ]
    uses: list[int] = [index for index, line in enumerate(lines) if line.startswith("use ")]
    headers: list[int] = [index for index, line in enumerate(lines) if line.startswith("//!")]
    at: int = uses[-1] + 1 if uses else (headers[-1] + 1 if headers else 0)
    prefix: list[str] = [] if uses else [""]
    return "\n".join([*lines[:at], *prefix, *wanted, *lines[at:]])


def write_constants(*, directory: Path, values: dict[str, str]) -> None:
    """Add the missing `values` to the domain's constants.rs and declare the module."""
    path: Path = directory / CONSTANTS_FILE
    text: str = path.read_text() if path.exists() else f"//! Constants of {directory.name}.\n"
    existing: dict[str, str] = dict(re.findall(r"const (\w+): &str = (\"[^\"]*\");", text))
    added: list[str] = []
    for name, literal in sorted(values.items()):
        if existing.get(name, literal) != literal:
            raise TransformError(f"{name} already names {existing[name]}, not {literal}")
        if name not in existing:
            added.append(
                f"\n/// The text {literal}, as comparisons test it.\n"
                f"pub(crate) const {name}: &str = {literal};\n"
            )
    path.write_text(text + "".join(added))
    _declare(module_file=directory / MOD_FILE)


def _declare(*, module_file: Path) -> None:
    lines: list[str] = module_file.read_text().split("\n")
    declaration: str = f"pub(crate) mod {CONSTANTS_MODULE};"
    if any(re.match(rf"(pub(\(\w+\))? )?mod {CONSTANTS_MODULE};", line) for line in lines):
        return
    modules: list[tuple[int, str]] = [
        (index, match.group(1))
        for index, line in enumerate(lines)
        if (match := re.search(r"\bmod (\w+);", line)) is not None
    ]
    after: list[int] = [index for index, name in modules if name < CONSTANTS_MODULE]
    at: int = after[-1] + 1 if after else modules[0][0]
    module_file.write_text("\n".join([*lines[:at], declaration, *lines[at:]]))
