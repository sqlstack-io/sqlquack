"""`Splitter`: moves a crate's items as placements say and re-points every path to them."""

from __future__ import annotations

from pathlib import Path

from scripts.fensu._helpers.rust.items import parse_items
from scripts.fensu._helpers.rust.use_trees import use_leaves
from scripts.fensu._helpers.splitting.split import (
    destination_module,
    follows_path_or_dot,
    innermost,
    is_macro_call,
    is_serde_path,
    item_key,
    leaf_module,
    leaf_text,
    path_at,
)
from scripts.fensu.classes.crate import Crate
from scripts.fensu.constants import (
    CLOSE_PAREN,
    CRATE,
    DROP,
    FN,
    IDENT,
    IMPL,
    MOD,
    NAMED_KINDS,
    OPEN_PAREN,
    PATH_SEPARATOR,
    PUB,
    SELF_OR_SUPER,
    SEMICOLON,
    SKIPPED_KINDS,
    STRING,
    SUPER,
    TRAIT,
    TRIVIA_KINDS,
    USE,
    VISIBILITY_TOKENS,
)
from scripts.fensu.exceptions import TransformError
from scripts.fensu.models import (
    Definition,
    Destination,
    External,
    Item,
    Piece,
    Placement,
    Replacement,
    SourceModule,
    Token,
    UseLeaf,
)


class Splitter:
    """Moves a crate's items as placements say, re-pointing every path that names them."""

    def __init__(self, *, crate: Crate, src: Path, placements: dict[str, Placement]) -> None:
        self.crate: Crate = crate
        self.src: Path = src
        self.placements: dict[str, Placement] = placements
        self.destinations: dict[str, Destination] = {}
        self.new_module: dict[tuple[tuple[str, ...], str], tuple[str, ...]] = {}
        self.moved_files: set[Path] = set()

    def plan(self) -> None:
        """Assign every item of every placed file to its destination."""
        for module in list(self.crate.modules.values()):
            if module.inline is not None:
                continue
            placement: Placement | None = self.placements.get(
                module.file.relative_to(self.src).as_posix()
            )
            if placement is not None:
                self.moved_files.add(module.file)
                self._place_module(module=module, placement=placement)

    def _place_module(self, *, module: SourceModule, placement: Placement) -> None:
        overrides: dict[str, str] = dict(placement.overrides)
        methods: dict[str, tuple[tuple[str, str], ...]] = dict(placement.methods)
        starts: dict[str, str] = dict(placement.sections)
        current: str = placement.sections[0][1]
        seen: dict[str, int] = {}
        for item in module.items:
            key: str = item_key(item)
            seen[key] = seen.get(key, 0) + 1
            numbered: str = f"{key}#{seen[key]}"
            current = (
                starts.get(numbered, starts.get(key, current))
                if item.kind not in SKIPPED_KINDS
                else current
            )
            if item.kind in SKIPPED_KINDS or (item.kind == MOD and item.body_start < 0):
                continue
            target: str = overrides.get(numbered, overrides.get(key, current))
            if target == DROP:
                continue
            if item.kind == MOD and item.body_start >= 0:
                for member in module.children[item.name].items:
                    if member.kind not in SKIPPED_KINDS:
                        self._add(
                            module=module.children[item.name],
                            item=member,
                            target=target,
                            inner_test=True,
                        )
                continue
            split: tuple[tuple[str, str], ...] | None = methods.get(numbered, methods.get(key))
            if item.kind == IMPL and split is not None:
                self._place_methods(module=module, item=item, split=split, default=target)
                continue
            self._add(module=module, item=item, target=target)

    def _place_methods(
        self, *, module: SourceModule, item: Item, split: tuple[tuple[str, str], ...], default: str
    ) -> None:
        starts: dict[str, str] = dict(split)
        current: str = default
        for member in parse_items(
            text=module.text, tokens=module.tokens, start=item.body_start, end=item.body_end
        ):
            if member.kind == FN and member.name in starts:
                current = starts[member.name]
            self._add(module=module, item=member, target=current, impl_item=item)

    def _add(
        self,
        *,
        module: SourceModule,
        item: Item,
        target: str,
        inner_test: bool = False,
        impl_item: Item | None = None,
    ) -> None:
        impl_header: str = (
            module.text[impl_item.start : impl_item.body_start] if impl_item is not None else ""
        )
        destination: Destination = self.destinations.setdefault(target, Destination(file=target))
        if item.kind in NAMED_KINDS and not impl_header and not inner_test:
            self.new_module[(module.path, item.name)] = destination_module(target)
        destination.pieces.append(
            Piece(
                source=module,
                item=item,
                impl_header=impl_header,
                inner_test=inner_test,
                impl_item=impl_item,
            )
        )

    def root_exports(self) -> set[str]:
        """The names the crate root makes public, which dependents keep using as they are."""
        root: SourceModule = self.crate.root
        names: set[str] = {leaf.name for leaf, item in root.uses if item.visibility == PUB}
        names |= {name for name, item in root.defs.items() if item.visibility == PUB}
        return names

    def target_of(self, definition: Definition) -> tuple[str, ...]:
        """The module a definition lives in after the split."""
        return self.new_module.get((definition.module, definition.name), definition.module)

    def kept_module(self, module: SourceModule) -> tuple[str, ...]:
        """The path of a module that survives the split; a split module is an error."""
        if module.file in self.moved_files:
            raise TransformError(f"a path names split module {'::'.join(module.path)}")
        return module.path

    def rewrite(
        self,
        *,
        module: SourceModule,
        tokens: list[Token],
        crate_name: str = CRATE,
        uses: bool = True,
    ) -> tuple[str, set[str]]:
        """The text of `tokens` with crate paths re-pointed, and the names it takes from scope."""
        significant: list[int] = [
            i for i, token in enumerate(tokens) if token.kind not in TRIVIA_KINDS
        ]
        replacements: dict[int, Replacement] = {}
        used: set[str] = set()
        position: int = 0
        while position < len(significant):
            token: Token = tokens[significant[position]]
            here: SourceModule = innermost(module=module, offset=token.start)
            if token.kind == IDENT and token.text == USE:
                position, found = self._use_statement(
                    module=here,
                    tokens=tokens,
                    significant=significant,
                    position=position,
                    crate_name=crate_name if uses else "",
                )
                replacements.update(found)
                continue
            if token.kind == STRING and is_serde_path(
                tokens=tokens, significant=significant, position=position
            ):
                used.add(token.text.strip('"').split(PATH_SEPARATOR)[0])
            if token.kind == IDENT and not follows_path_or_dot(
                tokens=tokens, significant=significant, position=position
            ):
                next_position, found, name = self._path(
                    module=here,
                    tokens=tokens,
                    significant=significant,
                    position=position,
                    crate_name=crate_name,
                )
                replacements.update(found)
                used |= name
                position = next_position
                continue
            position += 1
        return _applied(tokens=tokens, replacements=replacements), used

    def _use_statement(
        self,
        *,
        module: SourceModule,
        tokens: list[Token],
        significant: list[int],
        position: int,
        crate_name: str,
    ) -> tuple[int, dict[int, Replacement]]:
        start: int = position
        while start > 0 and tokens[significant[start - 1]].text in VISIBILITY_TOKENS:
            start -= 1
        end: int = position
        while tokens[significant[end]].text != SEMICOLON:
            end += 1
        if not crate_name:
            return end + 1, {}
        prefix: str = " ".join(tokens[i].text for i in significant[start:position])
        prefix = (
            prefix.replace("( ", OPEN_PAREN).replace(" (", OPEN_PAREN).replace(" )", CLOSE_PAREN)
        )
        leaves: list[UseLeaf] = use_leaves(
            tokens=[tokens[i] for i in significant[position : end + 1]]
        )
        rewritten: list[str] = [
            self.use_path(module=module, leaf=leaf, crate_name=crate_name) for leaf in leaves
        ]
        if rewritten == [leaf_text(leaf) for leaf in leaves]:
            return end + 1, {}
        text: str = "\n".join(f"{prefix + ' ' if prefix else ''}use {path};" for path in rewritten)
        return end + 1, {significant[start]: Replacement(stop=significant[end], text=text)}

    def _path(
        self,
        *,
        module: SourceModule,
        tokens: list[Token],
        significant: list[int],
        position: int,
        crate_name: str,
    ) -> tuple[int, dict[int, Replacement], set[str]]:
        segments, last = path_at(tokens=tokens, significant=significant, position=position)
        index: int = significant[position]
        rooted: bool = segments[0] == crate_name or segments[0] in SELF_OR_SUPER
        if rooted and len(segments) > 1:
            if crate_name == CRATE or segments[0] == crate_name:
                rewritten: str | None = self._rewrite_path(
                    module=module, segments=segments, crate_name=crate_name
                )
                if rewritten is not None:
                    return (
                        last + 1,
                        {index: Replacement(stop=significant[last], text=rewritten)},
                        set(),
                    )
            return last + 1, {}, set()
        if len(segments) > 1 and crate_name == CRATE:
            head: Definition | SourceModule | External | None = self.crate.lookup(
                module=module, name=segments[0]
            )
            if isinstance(head, SourceModule):
                rewritten = self._rewrite_path(
                    module=module, segments=segments, crate_name=crate_name
                )
                if rewritten is None:
                    return last + 1, {}, set()
                return last + 1, {index: Replacement(stop=significant[last], text=rewritten)}, set()
        if is_macro_call(tokens=tokens, significant=significant, position=position):
            return position + 1, {}, set()
        return position + 1, {}, {tokens[index].text}

    def _rewrite_path(
        self, *, module: SourceModule, segments: tuple[str, ...], crate_name: str
    ) -> str | None:
        internal: tuple[str, ...] = (
            ("crate", *segments[1:]) if segments[0] == crate_name else segments
        )
        if crate_name != CRATE and segments[1] in self.root_exports():
            return None
        found: tuple[Definition | SourceModule | External, int] | None = self.crate.resolve_prefix(
            module=module, segments=internal
        )
        if found is None:
            return None
        target, length = found
        if isinstance(target, External):
            return "::".join((*target.path, *internal[length:]))
        if isinstance(target, Definition):
            new: tuple[str, ...] = (*self.target_of(target), target.name)
        else:
            if target.file in self.moved_files:
                raise TransformError(
                    f"path {'::'.join(segments)} in {'::'.join(module.path)} names a split module"
                )
            new = target.path
        if crate_name != CRATE:
            new = (crate_name, *new[1:])
        return "::".join((*new, *internal[length:]))

    def use_path(self, *, module: SourceModule, leaf: UseLeaf, crate_name: str = "crate") -> str:
        """One `use` leaf after the split; unchanged when it leaves the crate."""
        if crate_name != CRATE and (not leaf.path or leaf.path[0] != crate_name):
            return leaf_text(leaf)
        internal: tuple[str, ...] = (
            ("crate", *leaf.path[1:]) if leaf.path and leaf.path[0] == crate_name else leaf.path
        )
        if crate_name != CRATE and len(leaf.path) > 1 and leaf.path[1] in self.root_exports():
            return (
                "::".join((*leaf.path, "*"))
                if leaf.glob
                else (
                    "::".join(leaf.path)
                    if leaf.name == leaf.path[-1]
                    else f"{'::'.join(leaf.path)} as {leaf.name}"
                )
            )
        target: Definition | SourceModule | External | None = self.crate.resolve(
            module=module, segments=internal
        )
        if leaf.glob:
            if isinstance(target, SourceModule):
                self.kept_module(target)
            return "::".join((*leaf.path, "*"))
        if target is None or isinstance(target, External):
            path: tuple[str, ...] = leaf.path
        elif isinstance(target, Definition):
            path = (*self.target_of(target), target.name)
        else:
            path = self.kept_module(target)
        if crate_name != CRATE and path[0] == CRATE:
            path = (crate_name, *path[1:])
        text: str = "::".join(path)
        return text if leaf.name == path[-1] else f"{text} as {leaf.name}"

    def scope_imports(
        self, *, module: SourceModule, used: set[str], defined: set[str], here: tuple[str, ...]
    ) -> set[str]:
        """The `use` paths a piece of `module` may need once moved to module `here`."""
        imports: set[str] = set()
        for leaf, item in self._scope_leaves(module=module):
            if leaf.glob:
                continue
            owner: SourceModule = leaf_module(module=module, item=item, crate=self.crate)
            target: Definition | SourceModule | External | None = self.crate.resolve(
                module=owner, segments=leaf.path
            )
            if target is None or isinstance(target, External):
                imports.add(
                    "::".join(leaf.path)
                    + ("" if leaf.name == leaf.path[-1] else f" as {leaf.name}")
                )
        for name in sorted(used - defined):
            found: Definition | SourceModule | External | None = self.crate.lookup(
                module=module, name=name
            )
            if isinstance(found, Definition):
                text: str | None = self._definition_use(target=found, name=name, here=here)
                if text:
                    imports.add(text)
        for name, definition in self._scope_traits(module=module).items():
            if name not in defined:
                text = self._definition_use(target=definition, name=name, here=here)
                if text:
                    imports.add(text)
        return imports

    def _definition_use(
        self, *, target: Definition, name: str, here: tuple[str, ...]
    ) -> str | None:
        new: tuple[str, ...] = self.target_of(target)
        if new == here:
            return None
        text: str = "::".join((*new, target.name))
        return text if name == target.name else f"{text} as {name}"

    def _scope_leaves(self, *, module: SourceModule) -> list[tuple[UseLeaf, Item]]:
        leaves: list[tuple[UseLeaf, Item]] = list(module.uses)
        for leaf, _item in module.uses:
            if leaf.glob and leaf.path == (SUPER,) and module.parent is not None:
                leaves.extend(self._scope_leaves(module=module.parent))
        return leaves

    def _scope_traits(self, *, module: SourceModule) -> dict[str, Definition]:
        traits: dict[str, Definition] = {}
        for name, item in module.defs.items():
            if item.kind == TRAIT:
                traits[name] = Definition(module=module.path, name=name, kind="trait")
        for leaf, _item in module.uses:
            target: Definition | SourceModule | External | None = self.crate.resolve(
                module=module, segments=leaf.path
            )
            if leaf.glob and isinstance(target, SourceModule):
                for name, definition in self._scope_traits(module=target).items():
                    traits.setdefault(name, definition)
            elif isinstance(target, Definition) and target.kind == TRAIT:
                traits.setdefault(leaf.name, target)
        return traits


def _applied(*, tokens: list[Token], replacements: dict[int, Replacement]) -> str:
    pieces: list[str] = []
    cursor: int = 0
    while cursor < len(tokens):
        replacement: Replacement | None = replacements.get(cursor)
        if replacement is not None:
            pieces.append(replacement.text)
            cursor = replacement.stop + 1
            continue
        pieces.append(tokens[cursor].text)
        cursor += 1
    return "".join(pieces)
