"""Direct wrapper for the workspace lockfile guard."""

from __future__ import annotations

from scripts.workspace_locks.main.check import run_workspace_locks


def main(argv: list[str] | None = None) -> int:
    """Fail when a cargo workspace's committed lockfile does not hold its dependencies."""
    return run_workspace_locks(argv)


if __name__ == "__main__":
    raise SystemExit(main())
