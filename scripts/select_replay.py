"""Direct wrapper for the replay scope: the replay a change needs, and the cache scope's digest."""

from __future__ import annotations

from scripts.replay_scope.main.scope import run_replay_scope


def main(argv: list[str] | None = None) -> int:
    """Print the replay a change needs or the replay cache's scope, or check the declared paths."""
    return run_replay_scope(argv)


if __name__ == "__main__":
    raise SystemExit(main())
