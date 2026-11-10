"""Workspace lockfile guard entry point."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.workspace_locks._helpers.locks import locked_resolution_error
from scripts.workspace_locks.constants import WORKSPACES


def run_workspace_locks(argv: list[str] | None = None) -> int:
    """Fail when a workspace's lockfile would have to change for cargo to resolve it."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--workspace", action="append", help="a workspace folder, repeatable")
    arguments: argparse.Namespace = parser.parse_args(argv)
    workspaces: tuple[str, ...] = tuple(arguments.workspace or WORKSPACES)
    errors: dict[str, str] = {
        workspace: locked_resolution_error(workspace=arguments.repository / workspace)
        for workspace in workspaces
    }
    stale: list[str] = [workspace for workspace, error in errors.items() if error]
    for workspace in stale:
        print(f"workspace locks: {workspace}: {errors[workspace]}", file=sys.stderr)
    print(
        f"workspace locks: {'FAILED' if stale else 'passed'}, "
        f"{len(workspaces)} workspaces resolved with --locked",
        file=sys.stderr,
    )
    return 1 if stale else 0
