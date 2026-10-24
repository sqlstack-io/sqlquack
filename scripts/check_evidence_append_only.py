"""Direct wrapper for the append-only evidence guard."""

from __future__ import annotations

from scripts.evidence_guard.main.check import run_evidence_guard


def main(argv: list[str] | None = None) -> int:
    """Fail when a change modifies or deletes a recorded result, or adds an invalid one."""
    return run_evidence_guard(argv)


if __name__ == "__main__":
    raise SystemExit(main())
