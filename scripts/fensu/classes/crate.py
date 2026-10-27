"""`Crate`: every module of one crate as written, and name resolution between them."""

from __future__ import annotations

from pathlib import Path

from scripts.fensu._helpers.rust.items import parse_items
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu._helpers.rust.use_trees import use_leaves
from scripts.fensu._helpers.splitting.crate_modules import child_file, parent_of, visible
from scripts.fensu.constants import CRATE, MOD, NAMED_KINDS, PATH_ROOTS, SELF_OR_SUPER, SUPER, USE
from scripts.fensu.models import Definition, External, Item, SourceModule, Token


class Crate:
    """Every module of one crate and name resolution between them."""

    def __init__(self, *, root: Path) -> None:
        self.modules: dict[tuple[str, ...], SourceModule] = {}
        self.root: SourceModule = self._load_file(path=("crate",), file=root, parent=None)

    def _load_file(
        self, *, path: tuple[str, ...], file: Path, parent: SourceModule | None
    ) -> SourceModule:
        text: str = file.read_text()
        tokens: list[Token] = tokenize(text=text)
        items: list[Item] = parse_items(text=text, tokens=tokens, start=0, end=len(text))
        module: SourceModule = SourceModule(
            path=path, file=file, text=text, tokens=tokens, items=items, parent=parent
        )
        return self._register(module=module)

    def _register(self, *, module: SourceModule) -> SourceModule:
        self.modules[module.path] = module
        for item in module.items:
            if item.kind in NAMED_KINDS:
                module.defs.setdefault(item.name, item)
            elif item.kind == USE:
                leaf_tokens: list[Token] = [
                    t for t in module.tokens if item.keyword_start <= t.start < item.end
                ]
                for leaf in use_leaves(tokens=leaf_tokens):
                    module.uses.append((leaf, item))
            elif item.kind == MOD:
                child_path: tuple[str, ...] = (*module.path, item.name)
                if item.body_start >= 0:
                    items: list[Item] = parse_items(
                        text=module.text,
                        tokens=module.tokens,
                        start=item.body_start,
                        end=item.body_end,
                    )
                    child: SourceModule = SourceModule(
                        path=child_path,
                        file=module.file,
                        text=module.text,
                        tokens=module.tokens,
                        items=items,
                        parent=module,
                        inline=item,
                    )
                    child = self._register(module=child)
                else:
                    child = self._load_file(
                        path=child_path,
                        file=child_file(module=module, name=item.name, item=item),
                        parent=module,
                    )
                module.children[item.name] = child
        return module

    def lookup(
        self,
        *,
        module: SourceModule,
        name: str,
        seen: frozenset[tuple[tuple[str, ...], str]] = frozenset(),
    ) -> Definition | SourceModule | External | None:
        """What `name` means in `module`: a definition, a module, or None (external or local)."""
        key: tuple[tuple[str, ...], str] = (module.path, name)
        if key in seen:
            return None
        seen = seen | {key}
        if name in module.defs:
            item: Item = module.defs[name]
            return Definition(module=module.path, name=name, kind=item.kind)
        if name in module.children:
            return module.children[name]
        for leaf, _item in module.uses:
            if not leaf.glob and leaf.name == name:
                resolved: Definition | SourceModule | External | None = self.resolve(
                    module=module, segments=leaf.path, seen=seen
                )
                if resolved is None and leaf.path[0] not in PATH_ROOTS:
                    return External(path=leaf.path)
                return resolved
        for leaf, _item in module.uses:
            if leaf.glob:
                target: Definition | SourceModule | External | None = self.resolve(
                    module=module, segments=leaf.path, seen=seen
                )
                if isinstance(target, SourceModule):
                    found: Definition | SourceModule | External | None = self.lookup(
                        module=target, name=name, seen=seen
                    )
                    if isinstance(found, Definition) and not visible(
                        definition=found,
                        item=self.modules[found.module].defs[found.name],
                        importer=module.path,
                    ):
                        continue
                    if found is not None:
                        return found
        return None

    def resolve(
        self,
        *,
        module: SourceModule,
        segments: tuple[str, ...],
        seen: frozenset[tuple[tuple[str, ...], str]] = frozenset(),
    ) -> Definition | SourceModule | External | None:
        """What a path means inside `module`; None when it leaves the crate."""
        current: SourceModule = module
        position: int = 0
        if not segments:
            return None
        if segments[0] == CRATE:
            current = self.root
            position = 1
        while position < len(segments) and segments[position] in SELF_OR_SUPER:
            if segments[position] == SUPER:
                current = parent_of(current)
            position += 1
        if position == len(segments):
            return current
        found: Definition | SourceModule | External | None = self.lookup(
            module=current, name=segments[position], seen=seen
        )
        position += 1
        while position < len(segments) and isinstance(found, SourceModule):
            found = self.lookup(module=found, name=segments[position], seen=seen)
            position += 1
        if position < len(segments):
            if isinstance(found, External):
                return External(path=(*found.path, *segments[position:]))
            return None
        return found

    def resolve_prefix(
        self, *, module: SourceModule, segments: tuple[str, ...]
    ) -> tuple[Definition | SourceModule | External, int] | None:
        """The longest leading part of a path that resolves, and how many segments it took."""
        for length in range(len(segments), 0, -1):
            found: Definition | SourceModule | External | None = self.resolve(
                module=module, segments=segments[:length]
            )
            if found is not None:
                return found, length
        return None
