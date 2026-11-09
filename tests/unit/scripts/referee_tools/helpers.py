"""A tiny compiled project imported as a referee corpus."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.referee_cases.main.generate import run_generate
from scripts.referee_tools._helpers.arities import catalogue_arities
from scripts.referee_tools._helpers.corpus_files import write_project_corpus
from scripts.referee_tools.main.pending import run_pending
from scripts.referee_tools.models import ProjectImport


def import_project(*, repository: Path, model_sql: str) -> tuple[list[object], str]:
    """Import a one-model project reading one declared source; its statements and summary."""
    project: Path = repository / "project"
    model: Path = project / "compiled" / "models" / "order_totals.sql"
    model.parent.mkdir(parents=True)
    model.write_text(model_sql, encoding="utf-8")
    dag: Path = project / "dag.json"
    nodes: list[dict[str, object]] = [
        {
            "kind": "source",
            "name": "shop__orders",
            "columns": [
                {"name": "order_id", "type": "NUMBER(38,0)", "nullable": False},
                {"name": "amount", "type": "NUMBER(10,2)"},
            ],
        },
        {"kind": "model", "name": "order_totals", "path": "models/order_totals.sql", "tags": []},
    ]
    dag.write_text(json.dumps({"nodes": nodes}), encoding="utf-8")
    summary: str = write_project_corpus(
        repository=repository,
        project=ProjectImport(
            name="shop",
            compiled=project / "compiled",
            dag=dag,
            origin="test",
            split_tag=None,
            every=1,
        ),
    )
    statements: list[object] = []
    for path in sorted((repository / "referee" / "corpora").glob("*.json")):
        statements.extend(json.loads(path.read_text(encoding="utf-8"))["statements"])
    return statements, summary


def pending_counts(
    *, repository: Path, selections: tuple[str, ...], source: str
) -> tuple[int, int, int]:
    """Pending cases and environments after each selection in turn, and `source`'s size."""
    run_generate(["--repository", str(repository)])
    for selection in selections:
        run_pending(["--repository", str(repository), "--select", selection])
    pending: Path = repository / "evidence" / "pending" / "referee"
    document: dict[str, list[object]] = json.loads(
        (repository / "referee" / source).read_text(encoding="utf-8")
    )
    return (
        len(list((pending / "cases").glob("*.json"))),
        len(list((pending / "environments").glob("*.json"))),
        len(document["statements"]),
    )


def arities_of(*, directory: Path, functions: list[dict[str, str]]) -> dict[str, object]:
    """The catalogue arities of `functions` written as a recorded catalogue."""
    path: Path = directory / "catalogue.json"
    path.write_text(json.dumps({"functions": functions}), encoding="utf-8")
    return dict(catalogue_arities(path))
