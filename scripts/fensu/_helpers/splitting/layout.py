"""Layout of a split: module docs, the new crate root and paths re-pointed outside the crate."""

from __future__ import annotations

from pathlib import Path

from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.classes.crate import Crate
from scripts.fensu.classes.splitter import Splitter
from scripts.fensu.constants import (
    CRATE,
    CRATE_DOC,
    LIB_FILE,
    MAIN_DIRECTORY,
    MOD,
    PUB,
    PUBLIC_MODULE,
    REWRITTEN_ROOTS,
    ROLE_DIRECTORIES,
    ROLE_DOCS,
    TESTS_STEM,
)
from scripts.fensu.models import Piece, SourceModule, Token


def default_doc(*, file: str, splitter: Splitter) -> str:
    """A one-line module doc for a destination: its entry's doc, its role, or its area."""
    parts: list[str] = file.removesuffix(".rs").split("/")
    stem: str = parts[-1].lstrip("_")
    area: str = " ".join(
        part.replace("_", " ") for part in parts[:-1] if part not in ROLE_DIRECTORIES
    )
    if MAIN_DIRECTORY in parts[:-1]:
        first_doc: str = _first_doc(splitter=splitter, file=file)
        return f"`{stem}`: {first_doc}" if first_doc else f"`{stem}`."
    if stem in ROLE_DOCS:
        return ROLE_DOCS[stem].format(area=area)
    if TESTS_STEM in parts[:-1]:
        return f"Tests of {stem.removeprefix('test_').replace('_', ' ')}."
    return f"{area[:1].upper() + area[1:]}: {stem.replace('_', ' ')}."


def _first_doc(*, splitter: Splitter, file: str) -> str:
    piece: Piece = splitter.destinations[file].pieces[0]
    lead: str = piece.source.text[piece.item.start : piece.item.header_start]
    for line in lead.split("\n"):
        stripped: str = line.strip()
        if stripped.startswith("///"):
            text: str = stripped.removeprefix("///").strip()
            return text[:1].lower() + text[1:]
    return ""


def lib_text(*, splitter: Splitter, crate: Crate, outputs: dict[str, str]) -> str:
    """The new crate root: module declarations and the same public re-exports."""
    root: SourceModule = crate.root
    lines: list[str] = [f"//! {CRATE_DOC}", "", "#![forbid(unsafe_code)]", ""]
    top: set[str] = {
        relative.split("/")[0].removesuffix(".rs") for relative in outputs if relative != LIB_FILE
    }
    declarations: dict[str, str] = {}
    for item in root.items:
        if item.kind == MOD and root.children[item.name].file not in splitter.moved_files:
            declarations[item.name] = root.text[item.start : item.end]
    for name in sorted(top - set(declarations)):
        text: str = outputs.get(f"{name}/mod.rs", "")
        public: bool = PUBLIC_MODULE in text
        declarations[name] = f"{'pub' if public else 'pub(crate)'} mod {name};"
    lines.extend(declarations[name] for name in sorted(declarations))
    exports: list[str] = []
    for leaf, item in root.uses:
        if item.visibility == PUB:
            exports.append(f"pub use {splitter.use_path(module=root, leaf=leaf)};")
    for (module, name), new in splitter.new_module.items():
        if module == (CRATE,) and root.defs[name].visibility == PUB:
            exports.append(f"pub use {'::'.join((*new, name))};")
    return "\n".join(lines) + "\n\n" + "\n".join(sorted(exports)) + "\n"


def rewrite_dependents(*, splitter: Splitter, crate: Crate, repository: Path) -> int:
    """Re-point paths into the crate from its tests, other crates and the derive."""
    count: int = 0
    for root, crate_name, uses in REWRITTEN_ROOTS:
        for path in sorted((repository / root).rglob("*.rs")):
            text: str = path.read_text()
            if crate_name not in text:
                continue
            tokens: list[Token] = tokenize(text=text)
            rewritten: str = splitter.rewrite(
                module=crate.root, tokens=tokens, crate_name=crate_name, uses=uses
            )[0]
            if rewritten != text:
                path.write_text(rewritten)
                count += 1
    return count
