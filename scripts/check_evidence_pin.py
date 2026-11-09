"""Direct wrapper for the evidence pin check."""

from __future__ import annotations

from scripts.evidence_pin.main.check import run_pin_check


def main(argv: list[str] | None = None) -> int:
    """Fail when evidence.toml's pin is malformed, unverified, off main, or rewrites evidence."""
    return run_pin_check(argv)


if __name__ == "__main__":
    raise SystemExit(main())
