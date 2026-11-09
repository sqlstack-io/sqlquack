"""Direct wrapper: drop unused imports after a split, gating test-only ones on `cfg(test)`."""

from __future__ import annotations

from scripts.fensu.main.prune_imports import run_prune_imports


def main(argv: list[str] | None = None) -> int:
    """Rewrite the `use` lines that the library or its tests do not need."""
    return run_prune_imports(argv)


if __name__ == "__main__":
    raise SystemExit(main())
