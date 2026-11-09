"""Directory listings read from the project tree Fensu discovered."""

from __future__ import annotations

from fensu import RuleContext


def directory_entries(*, ctx: RuleContext, directory: str) -> dict[str, str]:
    """Return each immediate entry name of a discovered directory, mapped to its path."""
    return {
        str(path).rsplit("/", 1)[-1]: str(path) for path in ctx.project.tree.children(directory)
    }
