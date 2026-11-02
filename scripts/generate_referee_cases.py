"""Direct wrapper for the referee case generators."""

from __future__ import annotations

from scripts.referee_cases.main.generate import run_generate


def main(argv: list[str] | None = None) -> int:
    """Write the generated referee files, or check them with --check."""
    return run_generate(argv)


if __name__ == "__main__":
    raise SystemExit(main())
