"""Direct wrapper: return internal errors instead of `unreachable!` (fensu FPRSH003)."""

from __future__ import annotations

from scripts.fensu.main.unreachable_errors import run_unreachable_errors


def main(argv: list[str] | None = None) -> int:
    """Rewrite each `unreachable!` in a `Result`-returning function under the given roots."""
    return run_unreachable_errors(argv)


if __name__ == "__main__":
    raise SystemExit(main())
