"""Shared code leaves the conversion's record of how the target holds values to the conversion."""

from __future__ import annotations

from fensu import AnalyzerId, Family, Fault, Project, RuleContext, rule

from scripts.fensu_policy._helpers.dialect_features import conversion_field_lines, path_dialect
from scripts.fensu_policy._helpers.layout_names import named_dialects
from scripts.fensu_policy.constants import (
    DIALECTS,
    FRONT_DOORS,
    SHARED_SYNTAX_CRATE,
    SOURCE_ROOT,
    TOOLS_ROOT,
)
from scripts.fensu_policy.models import SourceLine


@rule(
    code="XSQ009",
    family=Family.CUSTOM,
    slug="conversion-fields-unread",
    message="shared code leaves the conversion's holding fields to the conversion",
    remediation=(
        "Read a column's holding, typed_leaves, decimal_leaves, untainted, sourced, utc_offset, "
        "declared_offsets, widened or suffixed only in the source dialect's src/<dialect>/ code. "
        "Shared code asks the column's type through `DialectType` instead."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=False,
    options=(DIALECTS, FRONT_DOORS),
)
def conversion_fields_unread(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    front_doors: tuple[str, ...] = ctx.option(FRONT_DOORS)
    directories: dict[str, str] = {
        crate.name: str(crate.directory)
        for crate in ctx.rust.crates
        if crate.name not in front_doors
        and crate.name != SHARED_SYNTAX_CRATE
        and str(crate.directory).split("/")[0] != TOOLS_ROOT
        and not named_dialects(name=crate.name, dialects=dialects)
    }
    faults: list[Fault] = []
    for facts in ctx.rust.files:
        directory: str | None = directories.get(facts.crate_name)
        if directory is None:
            continue
        path: str = str(facts.file.path)
        parts: tuple[str, ...] = tuple(path.removeprefix(directory).strip("/").split("/"))
        if parts[0] != SOURCE_ROOT or path_dialect(parts=parts, dialects=dialects) is not None:
            continue
        lines: tuple[SourceLine, ...] = conversion_field_lines(source=facts.source)
        if lines:
            faults.append(
                ctx.fault_for(
                    path=ctx.repo_root / path,
                    line=lines[0].line,
                    column=0,
                    message=(
                        f"shared code reads a conversion holding field on {len(lines)} line(s), "
                        f"first `{lines[0].text}`"
                    ),
                )
            )
    return faults
