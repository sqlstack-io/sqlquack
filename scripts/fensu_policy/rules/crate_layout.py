"""Crate naming and dialect ownership across the Rust workspace."""

from __future__ import annotations

from fensu import AnalyzerId, Family, Fault, Project, RuleContext, rule

from scripts.fensu_policy._helpers.dialect_features import feature_gated_dependencies
from scripts.fensu_policy._helpers.layout_names import (
    crate_module,
    crate_name_problem,
    dialect_branches,
    dialect_path_parts,
    enum_mismatch,
    enum_variants,
    parser_crate_dialects,
    unnamed_dependency_dialects,
    variant_map,
)
from scripts.fensu_policy._helpers.manifests import crate_features
from scripts.fensu_policy.constants import (
    DIALECT_ENUM,
    DIALECT_VARIANTS,
    DIALECTS,
    FRONT_DOORS,
    ROLES,
    RUNTIME_DEPENDENCY_KINDS,
    SHARED_SYNTAX_CRATE,
    SHARED_SYNTAX_MODULES,
)
from scripts.fensu_policy.models import CrateDependencies, SourceLine


@rule(
    code="XSQ001",
    family=Family.CUSTOM,
    slug="crate-name-shape",
    message="crate names read sqlquack-<role>[-<dialect>], and tools end in -check",
    remediation=(
        "Rename the crate and its folder so the role comes first and one dialect after it, "
        "for example sqlquack-types-snowflake; a conversion crate names only its source, since "
        "every conversion targets DuckDB; name a development tool sqlquack-<subject>-check."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=True,
    options=(ROLES, DIALECTS),
)
def crate_name_shape(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    roles: tuple[str, ...] = ctx.option(ROLES)
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    faults: list[Fault] = []
    for crate in ctx.rust.crates:
        problem: str | None = crate_name_problem(
            name=crate.name, directory=str(crate.directory), roles=roles, dialects=dialects
        )
        if problem is not None:
            faults.append(
                ctx.path_fault(path=ctx.repo_root / str(crate.manifest_path), message=problem)
            )
    return faults


@rule(
    code="XSQ002",
    family=Family.CUSTOM,
    slug="dialect-named-dependents",
    message=(
        "a crate that uses a dialect-named crate names that dialect, or gates the dependency "
        "behind a feature of the same name"
    ),
    remediation=(
        "Reach the dialect through sqlquack-parse or another front door, move the code into a "
        "crate named for the dialect, or, in a multi-dialect role crate, keep it in src/<dialect>/ "
        "and make the dependency optional, turned on only by the <dialect> feature."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=False,
    options=(DIALECTS, FRONT_DOORS),
)
def dialect_named_dependents(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    front_doors: tuple[str, ...] = ctx.option(FRONT_DOORS)
    faults: list[Fault] = []
    for crate in ctx.rust.crates:
        dependencies: CrateDependencies = CrateDependencies(
            name=crate.name,
            directory=str(crate.directory),
            runtime_dependencies=tuple(
                dependency.package_name
                for dependency in crate.dependencies
                if dependency.local_crate_identity is not None
                and RUNTIME_DEPENDENCY_KINDS.intersection(dependency.kinds)
            ),
        )
        gated: frozenset[str] = feature_gated_dependencies(
            features=crate_features(ctx=ctx, manifest_path=str(crate.manifest_path)),
            dialects=dialects,
        )
        for dependency, dialect in unnamed_dependency_dialects(
            crate=dependencies, dialects=dialects, front_doors=front_doors, gated=gated
        ):
            faults.append(
                ctx.path_fault(
                    path=ctx.repo_root / str(crate.manifest_path),
                    message=f"`{crate.name}` uses `{dependency}` but does not name `{dialect}`",
                )
            )
    return faults


@rule(
    code="XSQ003",
    family=Family.CUSTOM,
    slug="dialect-enum-complete",
    message="the shared Dialect enum has exactly one variant per sqlquack-syntax-<dialect> crate",
    remediation=(
        "Add a variant for every parser crate and remove variants no parser crate owns, so "
        "sqlquack-parse can dispatch every dialect from one enum."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=True,
    options=(DIALECTS, DIALECT_VARIANTS),
)
def dialect_enum_complete(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    parser_dialects: tuple[str, ...] = parser_crate_dialects(
        crate_names=tuple(crate.name for crate in ctx.rust.crates), dialects=dialects
    )
    variants_by_dialect: dict[str, str] = variant_map(entries=ctx.option(DIALECT_VARIANTS))
    for facts in ctx.rust.files:
        if facts.crate_name != SHARED_SYNTAX_CRATE or facts.test:
            continue
        variants: tuple[str, ...] | None = enum_variants(source=facts.source, name=DIALECT_ENUM)
        if variants is None:
            continue
        missing, extra = enum_mismatch(
            variants=variants,
            parser_dialects=parser_dialects,
            variants_by_dialect=variants_by_dialect,
        )
        problems: list[str] = []
        if missing:
            problems.append(f"lacks {', '.join(missing)}")
        if extra:
            problems.append(f"has {', '.join(extra)} without a parser crate")
        if not problems:
            return []
        return [
            ctx.path_fault(
                path=ctx.repo_root / str(facts.file.path),
                message=f"enum {DIALECT_ENUM} {' and '.join(problems)}",
            )
        ]
    shared: list[str] = [
        str(crate.manifest_path) for crate in ctx.rust.crates if crate.name == SHARED_SYNTAX_CRATE
    ]
    return [
        ctx.path_fault(
            path=ctx.repo_root / path,
            message=f"{SHARED_SYNTAX_CRATE} defines no enum {DIALECT_ENUM}",
        )
        for path in shared
    ]


@rule(
    code="XSQ004",
    family=Family.CUSTOM,
    slug="shared-syntax-dialect-free",
    message="the shared sqlquack-syntax crate holds only shared modules, with no dialect in them",
    remediation=(
        "Move parsers and parse modes into the dialect's sqlquack-syntax-<dialect> crate and the "
        "dispatch into sqlquack-parse; the shared crate keeps only the tree, tokens, spans and "
        "diagnostics every parser uses, and never branches on a dialect."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=True,
    options=(DIALECTS, DIALECT_VARIANTS, SHARED_SYNTAX_MODULES),
)
def shared_syntax_dialect_free(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    allowed: tuple[str, ...] = ctx.option(SHARED_SYNTAX_MODULES)
    variants: tuple[str, ...] = tuple(variant_map(entries=ctx.option(DIALECT_VARIANTS)).values())
    directories: dict[str, str] = {crate.name: str(crate.directory) for crate in ctx.rust.crates}
    shared: str | None = directories.get(SHARED_SYNTAX_CRATE)
    if shared is None:
        return []
    misplaced: set[tuple[str, str]] = set()
    faults: list[Fault] = []
    for facts in ctx.rust.files:
        if facts.crate_name != SHARED_SYNTAX_CRATE:
            continue
        parts: tuple[str, ...] = tuple(
            str(facts.file.path).removeprefix(shared).strip("/").split("/")
        )
        section, module = crate_module(parts=parts)
        if module not in allowed:
            misplaced.add((section, module))
            continue
        named: tuple[str, ...] = dialect_path_parts(parts=parts, dialects=dialects)
        if named:
            faults.append(
                ctx.path_fault(
                    path=ctx.repo_root / str(facts.file.path),
                    message=f"file in the shared syntax crate is named for {', '.join(named)}",
                )
            )
            continue
        if enum_variants(source=facts.source, name=DIALECT_ENUM) is not None:
            continue
        branches: tuple[SourceLine, ...] = dialect_branches(
            source=facts.source, enum_name=DIALECT_ENUM, variants=variants
        )
        if branches:
            faults.append(
                ctx.fault_for(
                    path=ctx.repo_root / str(facts.file.path),
                    line=branches[0].line,
                    column=0,
                    message=(
                        f"the shared syntax crate branches on a dialect {len(branches)} times, "
                        f"first `{branches[0].text}`"
                    ),
                )
            )
    faults.extend(
        ctx.path_fault(
            path=ctx.repo_root / shared / section / module,
            message=f"`{section}/{module}` is not one of the shared syntax crate's modules",
        )
        for section, module in sorted(misplaced)
    )
    return faults
