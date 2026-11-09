"""Direct wrapper for the referee pending script."""

from __future__ import annotations

from scripts.referee_tools.main.pending import run_pending


def main(argv: list[str] | None = None) -> int:
    """Write pending referee files for the recorder."""
    return run_pending(argv)


if __name__ == "__main__":
    raise SystemExit(main())
