"""Multi-dialect role crates keep each dialect in a src/<dialect>/ folder behind its own feature."""

from __future__ import annotations

from pathlib import Path

from fensu import AnalyzerId, Family, Fault, Project, RuleContext, rule

from scripts.fensu_policy._helpers.dialect_features import (
    default_dialects,
    folders_without_features,
    path_dialect,
    shared_dialect_lines,
)
from scripts.fensu_policy._helpers.layout_names import (
    dialect_path_parts,
    named_dialects,
    variant_map,
)
from scripts.fensu_policy._helpers.manifests import crate_features
from scripts.fensu_policy.constants import (
    CRATE_SEPARATOR,
    DIALECT_REGISTRIES,
    DIALECT_VARIANTS,
    DIALECTS,
    FRONT_DOORS,
    RUST_NAME_SEPARATOR,
    SHARED_SYNTAX_CRATE,
    SOURCE_ROOT,
    TOOLS_ROOT,
)
from scripts.fensu_policy.models import CrateFeatures, DialectVocabulary, SourceLine


@rule(
    code="XSQ007",
    family=Family.CUSTOM,
    slug="dialect-folders-gated",
    message=(
        "dialect code lives only in src/<dialect>/ behind a feature of the same name, and shared "
        "code names no dialect"
    ),
    remediation=(
        "Move the dialect's code into src/<dialect>/, declare it as "
        '#[cfg(feature = "<dialect>")] mod <dialect>;, add the <dialect> feature, keep dialects '
        "out of the default features, and have shared code ask the dialect's profile through the "
        "registry instead of naming the dialect."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=False,
    options=(DIALECTS, DIALECT_VARIANTS, FRONT_DOORS, DIALECT_REGISTRIES),
)
def dialect_folders_gated(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    front_doors: tuple[str, ...] = ctx.option(FRONT_DOORS)
    registries: tuple[str, ...] = ctx.option(DIALECT_REGISTRIES)
    variants: dict[str, str] = {
        variant: dialect
        for dialect, variant in variant_map(entries=ctx.option(DIALECT_VARIANTS)).items()
    }
    dialect_crates: dict[str, str] = {
        crate.name.replace(CRATE_SEPARATOR, RUST_NAME_SEPARATOR): min(named)
        for crate in ctx.rust.crates
        if (named := named_dialects(name=crate.name, dialects=dialects))
    }
    directories: dict[str, str] = {
        crate.name: str(crate.directory)
        for crate in ctx.rust.crates
        if crate.name not in front_doors
        and crate.name != SHARED_SYNTAX_CRATE
        and str(crate.directory).split("/")[0] != TOOLS_ROOT
        and not named_dialects(name=crate.name, dialects=dialects)
    }
    shared_files: list[tuple[str, str, tuple[str, ...], str]] = []
    folders: dict[str, set[str]] = {name: set() for name in directories}
    for facts in ctx.rust.files:
        directory: str | None = directories.get(facts.crate_name)
        if directory is None:
            continue
        path: str = str(facts.file.path)
        parts: tuple[str, ...] = tuple(path.removeprefix(directory).strip("/").split("/"))
        if parts[0] != SOURCE_ROOT:
            continue
        dialect: str | None = path_dialect(parts=parts, dialects=dialects)
        if dialect is not None:
            folders[facts.crate_name].add(dialect)
        else:
            shared_files.append((facts.crate_name, path, parts, facts.source))
    faults: list[Fault] = []
    for crate in ctx.rust.crates:
        if crate.name not in directories:
            continue
        features: CrateFeatures = crate_features(ctx=ctx, manifest_path=str(crate.manifest_path))
        manifest: Path = ctx.repo_root / str(crate.manifest_path)
        faults.extend(
            ctx.path_fault(
                path=manifest,
                message=f"`{crate.name}` has src/{dialect}/ but no `{dialect}` feature",
            )
            for dialect in folders_without_features(
                folders=frozenset(folders[crate.name]), features=features
            )
        )
        faults.extend(
            ctx.path_fault(
                path=manifest,
                message=(
                    f"`{crate.name}`'s default features turn on `{dialect}`; the crate must "
                    "build with no dialect feature"
                ),
            )
            for dialect in default_dialects(features=features, dialects=dialects)
        )
    for crate_name, path, parts, source in shared_files:
        named: tuple[str, ...] = dialect_path_parts(parts=parts[1:], dialects=dialects)
        if named:
            faults.append(
                ctx.path_fault(
                    path=ctx.repo_root / path,
                    message=(
                        f"`{'/'.join(parts)}` is named for {', '.join(named)} but sits outside "
                        "every src/<dialect>/ folder"
                    ),
                )
            )
            continue
        lines: tuple[SourceLine, ...] = shared_dialect_lines(
            source=source,
            vocabulary=DialectVocabulary(
                dialects=dialects,
                variants=variants,
                folders=frozenset(folders[crate_name]),
                crates=dialect_crates,
            ),
            registry=any(path.startswith(f"{registry}/") for registry in registries),
        )
        if lines:
            faults.append(
                ctx.fault_for(
                    path=ctx.repo_root / path,
                    line=lines[0].line,
                    column=0,
                    message=(
                        f"shared code outside src/<dialect>/ names a dialect on {len(lines)} "
                        f"line(s), first `{lines[0].text}`"
                    ),
                )
            )
    return faults
