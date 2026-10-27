"""Direct wrapper: derive `Debug` on the data models (fensu FPRSS201) of the given Rust sources."""

from __future__ import annotations

from scripts.fensu.main.derive_debug import run_derive_debug


def main(argv: list[str] | None = None) -> int:
    """Add `Debug` to every struct and enum in the `models.rs` files under the given roots."""
    return run_derive_debug(argv)


if __name__ == "__main__":
    raise SystemExit(main())
