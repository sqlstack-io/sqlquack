"""Pure checks of how a crate keeps each dialect in its own feature-gated src/<dialect>/ folder."""

from __future__ import annotations

import re
import tomllib

from scripts.fensu_policy._helpers.layout_names import (
    named_dialects,
    strip_comments,
    strip_comments_and_strings,
)
from scripts.fensu_policy.constants import (
    ATTRIBUTE_LINE,
    CFG_FEATURE,
    CFG_WORD,
    DEFAULT_FEATURE,
    DEPENDENCIES_TABLE,
    DEPENDENCY_FEATURE_PREFIX,
    DIALECT_ENUM,
    DIALECT_MODULE_PATH,
    DIALECT_PROFILE_CONSTANT,
    DIALECT_VARIANT_PATH,
    FEATURE_SEPARATOR,
    FEATURES_TABLE,
    MODULE_DECLARATION,
    NESTED_PATH_PARTS,
    OPTIONAL_KEY,
    PROFILE_NAME_CALLS,
    RUST_SUFFIX,
    SOURCE_ROOT,
    WEAK_FEATURE_MARK,
)
from scripts.fensu_policy.models import CrateFeatures, DialectVocabulary, SourceLine


def _entries(*, raw: object) -> tuple[str, ...]:
    return tuple(str(entry) for entry in raw) if isinstance(raw, list) else ()


def read_crate_features(*, manifest_text: str) -> CrateFeatures:
    """Read the [features] table and the optional [dependencies] of a Cargo.toml."""
    manifest: dict[str, object] = tomllib.loads(manifest_text)
    raw_features: object = manifest.get(FEATURES_TABLE, {})
    raw_dependencies: object = manifest.get(DEPENDENCIES_TABLE, {})
    features: dict[str, tuple[str, ...]] = (
        {str(name): _entries(raw=entries) for name, entries in raw_features.items()}
        if isinstance(raw_features, dict)
        else {}
    )
    optional: frozenset[str] = (
        frozenset(
            str(name)
            for name, spec in raw_dependencies.items()
            if isinstance(spec, dict) and spec.get(OPTIONAL_KEY) is True
        )
        if isinstance(raw_dependencies, dict)
        else frozenset()
    )
    return CrateFeatures(features=features, optional_dependencies=optional)


def enabled_name(*, entry: str) -> str | None:
    """Return what a feature entry turns on; a weak `dep?/feature` entry turns on nothing."""
    if entry.startswith(DEPENDENCY_FEATURE_PREFIX):
        return entry.removeprefix(DEPENDENCY_FEATURE_PREFIX)
    name: str = entry.partition(FEATURE_SEPARATOR)[0]
    return None if name.endswith(WEAK_FEATURE_MARK) else name


def feature_closure(*, features: CrateFeatures, start: str) -> frozenset[str]:
    """Return every feature and dependency that turning on `start` turns on, including itself."""
    reached: set[str] = set()
    pending: list[str] = [start]
    while pending:
        name: str = pending.pop()
        if name in reached:
            continue
        reached.add(name)
        enabled: tuple[str | None, ...] = tuple(
            enabled_name(entry=entry) for entry in features.features.get(name, ())
        )
        pending.extend(item for item in enabled if item is not None)
    return frozenset(reached)


def default_dialects(*, features: CrateFeatures, dialects: tuple[str, ...]) -> tuple[str, ...]:
    """Return the dialect features the crate's default features turn on."""
    return tuple(sorted(feature_closure(features=features, start=DEFAULT_FEATURE) & set(dialects)))


def feature_gated_dependencies(
    *, features: CrateFeatures, dialects: tuple[str, ...]
) -> frozenset[str]:
    """Return the optional dialect-named dependencies only their own dialect feature turns on."""
    gated: set[str] = set()
    for dependency in features.optional_dependencies:
        own: frozenset[str] = named_dialects(name=dependency, dialects=dialects)
        enablers: frozenset[str] = frozenset(
            feature
            for feature in features.features
            if dependency in feature_closure(features=features, start=feature)
        )
        if len(own) == 1 and enablers and enablers <= own:
            gated.add(dependency)
    return frozenset(gated)


def path_dialect(*, parts: tuple[str, ...], dialects: tuple[str, ...]) -> str | None:
    """Return the dialect whose src/<dialect>/ folder or src/<dialect>.rs holds a crate path."""
    if len(parts) < NESTED_PATH_PARTS or parts[0] != SOURCE_ROOT:
        return None
    name: str = parts[1] if len(parts) > NESTED_PATH_PARTS else parts[1].removesuffix(RUST_SUFFIX)
    return name if name in dialects else None


def folders_without_features(
    *, folders: frozenset[str], features: CrateFeatures
) -> tuple[str, ...]:
    """Return the dialect folders that have no feature of the same name."""
    return tuple(sorted(folders - set(features.features)))


def _line_dialects(*, code: str, vocabulary: DialectVocabulary) -> frozenset[str]:
    found: set[str] = set()
    variants: str = "|".join(re.escape(variant) for variant in vocabulary.variants)
    if variants:
        pattern: str = DIALECT_VARIANT_PATH.format(enum=DIALECT_ENUM, variant=variants)
        found.update(vocabulary.variants[match] for match in re.findall(pattern, code))
    found.update(
        dialect
        for dialect in vocabulary.folders
        if re.search(DIALECT_MODULE_PATH.format(dialect=re.escape(dialect)), code)
    )
    found.update(
        dialect
        for crate, dialect in vocabulary.crates.items()
        if re.search(rf"\b{re.escape(crate)}\b", code)
    )
    return frozenset(found)


def _cfg_dialects(*, raw: str, code: str, dialects: tuple[str, ...]) -> frozenset[str]:
    if CFG_WORD not in code:
        return frozenset()
    return frozenset(name for name in CFG_FEATURE.findall(raw) if name in dialects)


def _declared_folders(*, code: str, folders: frozenset[str]) -> frozenset[str]:
    return frozenset(
        dialect
        for dialect in folders
        if re.search(MODULE_DECLARATION.format(dialect=re.escape(dialect)), code)
    )


def shared_dialect_lines(
    *, source: str, vocabulary: DialectVocabulary, registry: bool
) -> tuple[SourceLine, ...]:
    """Return shared lines naming a dialect, except gated folder modules and registry lines."""
    raw_lines: list[str] = strip_comments(source=source).splitlines()
    code_lines: list[str] = strip_comments_and_strings(source=source).splitlines()
    found: list[SourceLine] = []
    pending: SourceLine | None = None
    pending_gates: frozenset[str] = frozenset()
    for number, (raw, code) in enumerate(zip(raw_lines, code_lines, strict=True), start=1):
        if not code.strip():
            continue
        gates: frozenset[str] = _cfg_dialects(raw=raw, code=code, dialects=vocabulary.dialects)
        named: frozenset[str] = _line_dialects(code=code, vocabulary=vocabulary)
        attribute: bool = bool(ATTRIBUTE_LINE.match(code))
        if attribute and gates:
            pending = pending or SourceLine(line=number, text=raw.strip())
            pending_gates = pending_gates | gates
            continue
        if attribute:
            continue
        gating: frozenset[str] = pending_gates | gates
        declared: frozenset[str] = _declared_folders(code=code, folders=vocabulary.folders)
        gated_module: bool = len(declared) == 1 and declared == gating
        line_allowed: bool = (
            gated_module
            or (registry and (named | declared) <= gating)
            or not (named or gates or declared)
        )
        gate_allowed: bool = registry or gated_module
        found.extend([pending] if pending is not None and not gate_allowed else [])
        found.extend([] if line_allowed else [SourceLine(line=number, text=raw.strip())])
        pending, pending_gates = None, frozenset()
    found.extend([pending] if pending is not None and not registry else [])
    return tuple(sorted(set(found), key=lambda line: line.line))


def dialect_name_lines(*, source: str, dialects: tuple[str, ...]) -> tuple[SourceLine, ...]:
    """Return each line where a match starts across the whole code, so split chains are found."""
    constant: re.Pattern[str] = re.compile(
        DIALECT_PROFILE_CONSTANT.format(
            dialects="|".join(re.escape(dialect.upper()) for dialect in dialects)
        )
    )
    raw_lines: list[str] = strip_comments(source=source).splitlines()
    code: str = strip_comments_and_strings(source=source)
    starts: set[int] = set()
    for pattern in (*PROFILE_NAME_CALLS, constant):
        starts.update(match.start() for match in pattern.finditer(code))
    numbers: list[int] = sorted({code.count("\n", 0, start) + 1 for start in starts})
    return tuple(SourceLine(line=number, text=raw_lines[number - 1].strip()) for number in numbers)
