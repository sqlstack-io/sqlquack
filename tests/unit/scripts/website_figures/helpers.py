"""A coverage summary and stale website figures, written into a temporary directory."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from scripts.website_figures.main.figures import run_figures

SUMMARY: dict[str, Any] = {
    "total": {"forms": 3351, "verified": 3199},
    "verified_percent": 95.4640405849,
}
STALE_FIGURES: dict[str, str] = {
    "SYNTAX_VERIFIED": "2,388",
    "SYNTAX_FORMS": "3,351",
    "SYNTAX_PCT": "71.26%",
    "SYNTAX_BASIS": "2,388 forms verified, as of 6 November 2026.",
    "DUCKSHOP_MODELS": "1,000",
}


def written_files(*, tmp_path: Path, day: str) -> tuple[Path, Path]:
    """The summary file and the figures file after `write` replaced the stale figures."""
    summary: Path = tmp_path / "summary.json"
    figures: Path = tmp_path / "figures.json"
    summary.write_text(json.dumps(SUMMARY))
    figures.write_text(json.dumps(STALE_FIGURES))
    written: int = run_figures(["write", str(summary), "--figures", str(figures), "--day", day])
    assert written == 0
    return summary, figures
