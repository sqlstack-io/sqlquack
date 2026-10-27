"""The text of the files a split writes: moved items with their imports, and declaration files."""

from __future__ import annotations

from bisect import bisect_left
from pathlib import Path

from scripts.fensu._helpers.rust.items import parse_items
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu._helpers.splitting.split import bump_visibility, destination_module
from scripts.fensu.classes.splitter import Splitter
from scripts.fensu.constants import (
    DECLARATION_STEMS,
    HELPERS_DIRECTORY,
    INNER_ATTRIBUTE,
    MAIN_DIRECTORY,
    NAMED_KINDS,
    PRIVATE_PREFIX,
    PUB,
    TESTS_DIRECTORY_SUFFIX,
    TESTS_STEM,
    USE,
    VALUE_KINDS,
)
from scripts.fensu.models import Destination, Item, Piece, SourceModule, Token


def destination_text(*, splitter: Splitter, destination: Destination, doc: str) -> str:
    """A destination file: its doc line, the imports its pieces need, then the pieces in order."""
    here: tuple[str, ...] = destination_module(destination.file)
    defined: set[str] = {
        piece.item.name
        for piece in destination.pieces
        if piece.item.kind in NAMED_KINDS and not piece.impl_header
    }
    blocks: list[str] = []
    imports: set[str] = set()
    groups: list[list[Piece]] = []
    for piece in destination.pieces:
        if (
            groups
            and piece.impl_header
            and groups[-1][-1].impl_header == piece.impl_header
            and groups[-1][-1].source is piece.source
        ):
            groups[-1].append(piece)
        else:
            groups.append([piece])
    groups.sort(
        key=lambda group: 0 if group[0].item.kind in VALUE_KINDS and not group[0].impl_header else 1
    )
    for group in groups:
        first: Piece = group[0]
        texts: list[str] = []
        used: set[str] = set()
        for piece in group:
            text, names = splitter.rewrite(
                module=piece.source,
                tokens=_slice(module=piece.source, start=piece.item.start, end=piece.item.end),
            )
            if not piece.inner_test:
                text: str = bump_visibility(
                    text=text, visibility=_entry_visibility(destination.file)
                )
            texts.append(text)
            used |= names
        if first.impl_item is not None:
            header_text, names = splitter.rewrite(
                module=first.source,
                tokens=_slice(
                    module=first.source, start=first.impl_item.start, end=first.impl_item.body_start
                ),
            )
            used |= names
            blocks.append(header_text + "\n" + "\n\n".join(texts) + "\n}")
        else:
            blocks.extend(texts)
        imports |= splitter.scope_imports(
            module=first.source, used=used, defined=defined, here=here
        )
    lines: list[str] = [f"//! {doc}", ""]
    attributes: list[str] = []
    sources: dict[tuple[str, ...], SourceModule] = {
        piece.source.path: piece.source for piece in destination.pieces if not piece.inner_test
    }
    for source in sources.values():
        for item in source.items:
            text: str = source.text[item.start : item.end]
            if item.kind == INNER_ATTRIBUTE and text not in attributes:
                attributes.append(text)
    if attributes:
        lines.extend(attributes)
        lines.append("")
    external: list[str] = sorted(text for text in imports if not text.startswith("crate::"))
    internal: list[str] = sorted(text for text in imports if text.startswith("crate::"))
    for block in (external, internal):
        if block:
            lines.extend(f"use {text};" for text in block)
            lines.append("")
    return "\n".join(lines) + "\n" + "\n\n".join(blocks) + "\n"


def _entry_visibility(file: str) -> str:
    parts: list[str] = file.split("/")
    if parts[-1].startswith(PRIVATE_PREFIX) and MAIN_DIRECTORY in parts:
        return f"pub(in crate::{parts[0]})"
    return "pub(crate)"


def _slice(*, module: SourceModule, start: int, end: int) -> list[Token]:
    starts: list[int] = [token.start for token in module.tokens]
    return module.tokens[bisect_left(starts, start) : bisect_left(starts, end)]


def is_public(*, text: str) -> bool:
    """Whether a file declares a `pub` (not `pub(crate)`) top-level item."""
    tokens: list[Token] = tokenize(text=text)
    items: list[Item] = parse_items(text=text, tokens=tokens, start=0, end=len(text))
    return any(item.visibility == PUB and item.kind != USE for item in items)


def declaration_files(
    *, files: dict[str, str], docs: dict[str, str], kept: dict[str, str]
) -> dict[str, str]:
    """`mod.rs` for every new directory and `tests.rs` harnesses, from the files under `src/`."""
    directories: dict[str, set[str]] = {}
    for file in files:
        parts: list[str] = file.split("/")
        for depth in range(1, len(parts)):
            directories.setdefault("/".join(parts[:depth]), set())
        for depth in range(len(parts)):
            parent: str = "/".join(parts[:depth])
            directories.setdefault(parent, set()).add(parts[depth])
    public: dict[str, bool] = {}
    for file, text in files.items():
        public[file.removesuffix(".rs")] = is_public(text=text)
    outputs: dict[str, str] = {}
    for directory, children in sorted(directories.items()):
        if directory.endswith(TESTS_DIRECTORY_SUFFIX) or not directory or directory in kept:
            continue
        declarations: list[str] = []
        harness: list[str] = []
        for child in sorted(children):
            name: str = child.removesuffix(".rs")
            if name in DECLARATION_STEMS and child.endswith(".rs"):
                continue
            prefix: str = f"{directory}/{name}"
            if name == TESTS_STEM:
                for test in sorted(directories[prefix]):
                    harness.append(f'#[path = "tests/{test}"]\nmod {test.removesuffix(".rs")};')
                continue
            visible: bool = name != HELPERS_DIRECTORY and any(
                is_pub
                for key, is_pub in public.items()
                if key == prefix or key.startswith(prefix + "/")
            )
            restricted: str = f"pub(in crate::{directory.split('/')[0]})"
            visibility: str = (
                restricted
                if name.startswith("_") and name != HELPERS_DIRECTORY
                else ("pub" if visible else "pub(crate)")
            )
            declarations.append(f"{visibility} mod {name};")
        if harness:
            declarations.append("\n#[cfg(test)]\nmod tests;")
            outputs[f"{directory}/tests.rs"] = "\n".join(harness) + "\n"
        doc: str = docs.get(
            directory, directory.split("/")[-1].replace("_", " ").capitalize() + "."
        )
        outputs[f"{directory}/mod.rs"] = f"//! {doc}\n\n" + "\n".join(declarations) + "\n"
    return outputs


def write_outputs(*, src: Path, outputs: dict[str, str], removed: set[Path]) -> None:
    """Write every output and delete the split files no output replaced."""
    for path in removed:
        if path.relative_to(src).as_posix() not in outputs:
            path.unlink()
    for relative, text in outputs.items():
        target: Path = src / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
    for directory in sorted({path.parent for path in removed}, reverse=True):
        if directory.exists() and not any(directory.iterdir()):
            directory.rmdir()
