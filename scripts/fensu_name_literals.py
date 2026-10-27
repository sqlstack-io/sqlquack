"""Direct wrapper: name compared string literals in domain constants (fensu FPRSH007)."""

from __future__ import annotations

from scripts.fensu.main.name_literals import run_name_literals


def main(argv: list[str] | None = None) -> int:
    """Replace each compared literal under the crate source with a domain constant."""
    return run_name_literals(argv)


if __name__ == "__main__":
    raise SystemExit(main())
