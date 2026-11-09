"""Direct wrapper for the tracked-content guard."""

from __future__ import annotations

from scripts.clutter_guard.main.check import run_clutter_guard


def main(argv: list[str] | None = None) -> int:
    """Fail when a tracked path or text breaks the public-repository content rules."""
    return run_clutter_guard(argv)


if __name__ == "__main__":
    raise SystemExit(main())
