"""Direct wrapper: remove non-doc comments (fensu FPRSH002) from the given Rust sources."""

from __future__ import annotations

from scripts.fensu.main.strip_comments import run_strip_comments


def main(argv: list[str] | None = None) -> int:
    """Rewrite every Rust file under the given roots without non-doc comments."""
    return run_strip_comments(argv)


if __name__ == "__main__":
    raise SystemExit(main())
