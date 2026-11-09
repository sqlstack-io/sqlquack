"""Direct wrapper: split the expression models into one domain per syntax area."""

from __future__ import annotations

from scripts.fensu.main.split_models import run_split_models


def main(argv: list[str] | None = None) -> int:
    """Split the expression models and re-point every path to them."""
    return run_split_models(argv)


if __name__ == "__main__":
    raise SystemExit(main())
