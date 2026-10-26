"""Direct wrapper for the polyglot runtime-dependency guard."""

from __future__ import annotations

from scripts.polyglot_guard.main.check import run_polyglot_guard


def main(argv: list[str] | None = None) -> int:
    """Fail when a crate other than the dev-only check reaches polyglot at runtime."""
    return run_polyglot_guard(argv)


if __name__ == "__main__":
    raise SystemExit(main())
