"""Shared code asks a dialect's profile a question instead of comparing the dialect's name."""

from __future__ import annotations

from fensu import AnalyzerId, Family, Fault, Project, RuleContext, rule

from scripts.fensu_policy._helpers.dialect_features import dialect_name_lines, path_dialect
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
    code="XSQ008",
    family=Family.CUSTOM,
    slug="dialect-names-uncompared",
    message="shared code asks the dialect profile a question instead of comparing its name",
    remediation=(
        "Add a method to the dialect profile that answers the question the comparison stands "
        "for, answer it in each src/<dialect>/ profile, and call it here instead of reading "
        "a profile's name (`<receiver>profile.name()` or `DialectProfile::name(...)`) or a "
        "`<DIALECT>_PROFILE` name."
    ),
    analyzers=(AnalyzerId.RUST,),
    cacheable=False,
    options=(DIALECTS, FRONT_DOORS),
)
def dialect_names_uncompared(*, project: Project, ctx: RuleContext) -> list[Fault]:
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
        lines: tuple[SourceLine, ...] = dialect_name_lines(source=facts.source, dialects=dialects)
        if lines:
            faults.append(
                ctx.fault_for(
                    path=ctx.repo_root / path,
                    line=lines[0].line,
                    column=0,
                    message=(
                        f"shared code compares a dialect's name on {len(lines)} line(s), first "
                        f"`{lines[0].text}`"
                    ),
                )
            )
    return faults
