"""Dialect and vocabulary naming of repository scripts."""

from __future__ import annotations

from fensu import AnalyzerId, Family, Fault, Project, RuleContext, rule

from scripts.fensu_policy._helpers.layout_names import catalog_spellings, unnamed_defaults
from scripts.fensu_policy._helpers.tree_entries import directory_entries
from scripts.fensu_policy.constants import DIALECTS, NAMED_DIRECTORIES


@rule(
    code="XSQ005",
    family=Family.CUSTOM,
    slug="no-unnamed-default-dialect",
    message="a script for one dialect names it, even when a sibling script names another",
    remediation=(
        "Add the dialect to the name, for example build_syntax_catalogue_snowflake.py beside "
        "build_syntax_catalogue_duckdb.py; no dialect is the unnamed default."
    ),
    analyzers=(AnalyzerId.PYTHON,),
    cacheable=True,
    options=(DIALECTS, NAMED_DIRECTORIES),
)
def no_unnamed_default_dialect(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    dialects: tuple[str, ...] = ctx.option(DIALECTS)
    faults: list[Fault] = []
    for directory in ctx.option(NAMED_DIRECTORIES):
        entries: dict[str, str] = directory_entries(ctx=ctx, directory=directory)
        faults.extend(
            ctx.path_fault(
                path=entries[entry],
                message=f"`{entry}` names no dialect, but a sibling does the same job for one",
            )
            for entry in unnamed_defaults(entries=tuple(entries), dialects=dialects)
        )
    return faults


@rule(
    code="XSQ006",
    family=Family.CUSTOM,
    slug="catalogue-spelling",
    message="script names spell the documented catalogue as catalogue",
    remediation="Rename the script or package from catalog to catalogue.",
    analyzers=(AnalyzerId.PYTHON,),
    cacheable=True,
    options=(NAMED_DIRECTORIES,),
)
def catalogue_spelling(*, project: Project, ctx: RuleContext) -> list[Fault]:
    del project
    faults: list[Fault] = []
    for directory in ctx.option(NAMED_DIRECTORIES):
        entries: dict[str, str] = directory_entries(ctx=ctx, directory=directory)
        faults.extend(
            ctx.path_fault(path=entries[entry], message=f"`{entry}` spells it catalog")
            for entry in catalog_spellings(entries=tuple(entries))
        )
    return faults
