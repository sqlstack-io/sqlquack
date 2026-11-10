"""What a change replays and checks, the replay cache's scope, or the declared paths' check."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.replay_scope._helpers.decisions import change_plan, scope_digest, unmatched
from scripts.replay_scope.constants import CHECK_COMMAND, DIGEST_COMMAND, SCOPE_COMMAND


def run_replay_scope(argv: list[str] | None = None) -> int:
    """Run one of the scope, digest and check commands."""
    arguments: argparse.Namespace = _parser().parse_args(argv)
    root: Path = arguments.root
    if arguments.command == DIGEST_COMMAND:
        print(scope_digest(root=root))
        return 0
    if arguments.command == CHECK_COMMAND:
        missing: list[str] = unmatched(root=root)
        for prefix in missing:
            print(f"replay scope: {prefix} matches no tracked file", file=sys.stderr)
        print(f"replay scope: {'FAILED' if missing else 'passed'}", file=sys.stderr)
        return 1 if missing else 0
    plan: dict[str, str] = change_plan(root=root, base=arguments.base)
    print(f"replay scope: {plan} against {arguments.base}", file=sys.stderr)
    print("\n".join(f"{key}={value}" for key, value in plan.items()))
    return 0


def _parser() -> argparse.ArgumentParser:
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command",
        choices=(SCOPE_COMMAND, DIGEST_COMMAND, CHECK_COMMAND),
        help="scope: replay=none|changed|full, code= and site=true|false; digest: the replay "
        "cache's scope; check: fail when a declared path matches no tracked file",
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--base", default="origin/main", help="the commit the change merges into")
    return parser
