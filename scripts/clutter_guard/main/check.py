"""Agent clutter guard entry point."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

from scripts.clutter_guard._helpers.content import file_findings
from scripts.clutter_guard._helpers.git import tracked_paths
from scripts.clutter_guard._helpers.paths import path_findings
from scripts.clutter_guard.constants import CLUTTER_CLASSES
from scripts.clutter_guard.models import Finding


def run_clutter_guard(argv: list[str] | None = None) -> int:
    """Reject tracked paths and text that carry process vocabulary into the public repository."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--json", action="store_true", help="print the findings as JSON")
    arguments: argparse.Namespace = parser.parse_args(argv)
    root: Path = arguments.root
    paths: list[str] = tracked_paths(root=root)
    findings: list[Finding] = []
    for path in paths:
        findings.extend(path_findings(path=path))
        findings.extend(file_findings(root=root, path=path))
    findings.sort()
    counted: Counter[str] = Counter(finding.clutter_class for finding in findings)
    counts: dict[str, int] = {
        clutter_class: counted[clutter_class] for clutter_class in CLUTTER_CLASSES
    }
    if arguments.json:
        document: dict[str, object] = {
            "files": len(paths),
            "counts": counts,
            "findings": [
                {
                    "path": finding.path,
                    "line": finding.line,
                    "class": finding.clutter_class,
                    "snippet": finding.snippet,
                }
                for finding in findings
            ],
        }
        print(json.dumps(document, indent=2))
    else:
        for finding in findings:
            print(f"{finding.path}:{finding.line}: {finding.clutter_class}: {finding.snippet}")
        for clutter_class, count in counts.items():
            print(f"clutter guard: {clutter_class}: {count}")
        print(
            f"clutter guard: {'FAILED' if findings else 'passed'}, {len(findings)} findings in "
            f"{len(paths)} tracked files"
        )
    return 1 if findings else 0
