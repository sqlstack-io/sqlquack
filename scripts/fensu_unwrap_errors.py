"""Direct wrapper: propagate internal errors instead of unwrapping (fensu FPRSH010)."""

from __future__ import annotations

from scripts.fensu.main.unwrap_errors import run_unwrap_errors


def main(argv: list[str] | None = None) -> int:
    """Rewrite each `.unwrap()`/`.expect(..)` in a `Result` function under the given roots."""
    return run_unwrap_errors(argv)


if __name__ == "__main__":
    raise SystemExit(main())
