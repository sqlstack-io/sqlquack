"""A fake referee session and helpers writing pending referee files."""

from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path

from scripts.referee_cases.main.generate import run_generate
from scripts.referee_tools.main.pending import run_pending
from sqlquack_record.recording.exceptions import InternalCompileError
from sqlquack_record.recording.models import ColumnDescription, WarehouseFailure
from sqlquack_record.recording.types import SessionParameters
from sqlquack_record.referee._helpers.hashing import case_sha256, environment_sha256
from sqlquack_record.referee.main.load_referee import load_referee
from sqlquack_record.referee.main.record_referee import record_referee
from sqlquack_record.referee.models import RefereePending, RefereeSummary
from sqlquack_record.referee.types import RefereeSession

PARAMETERS: dict[str, bool | int | str] = {"TIMEZONE": "UTC"}
_COLUMNS: tuple[ColumnDescription, ...] = (
    ColumnDescription("ORDER_ID", 0, "FIXED", 38, 0, None, True),
)
_MISSING: WarehouseFailure = WarehouseFailure(
    errno=904,
    sqlstate="42000",
    message="SQL compilation error: error line 1 at position 7 invalid identifier "
    "'ORDER_IDD' in SHOP_DB.SCRATCH",
)
_SYNTAX: WarehouseFailure = WarehouseFailure(
    errno=1003,
    sqlstate="42000",
    message="SQL compilation error: syntax error line 1 at position 9 unexpected 'FROM'.",
)
_FAILURES: dict[str, WarehouseFailure] = {
    "SELECT order_idd FROM orders": _MISSING,
    "SELECT FROM orders": _SYNTAX,
}
_INTERNAL: frozenset[str] = frozenset({"SELECT 'INTERNAL' FROM orders"})


class FakeRefereeSession:
    """Compiles from scripted answers and logs what it was asked to do."""

    def __init__(self, *, log: list[str]) -> None:
        self._log: list[str] = log

    def warehouse_version(self) -> str:
        return "0.0.0-fake"

    def connector(self) -> str:
        return "fake-connector 0"

    def compile_failure(self, statement: str) -> WarehouseFailure | None:
        self._log.append("explain")
        for _ in _INTERNAL.intersection({statement}):
            raise InternalCompileError("Snowflake error 603 (SQLSTATE XX000) while compiling")
        return _FAILURES.get(statement)

    def describe(self, statement: str) -> tuple[ColumnDescription, ...] | WarehouseFailure:
        self._log.append("describe")
        return _COLUMNS

    def execute(self, statement: str) -> None:
        self._log.append("create")

    def builtin_functions(self) -> list[dict[str, str]]:
        self._log.append("show functions")
        return [
            {"name": "ABS", "arguments": "ABS(NUMBER) RETURN NUMBER", "is_builtin": "Y"},
            {"name": "ORDER_TOTAL", "arguments": "ORDER_TOTAL(NUMBER)", "is_builtin": "N"},
        ]

    def identity(self) -> tuple[tuple[str, str], ...]:
        return (("database", "SHOP_DB"), ("schema", "SCRATCH"), ("user", "ANALYST"))

    def session_parameters(self) -> dict[str, str]:
        return {"TIMEZONE": "UTC", "QUERY_TAG": "fake"}

    def close(self) -> None:
        self._log.append("close")


def write_pending(
    *,
    directory: Path,
    setup_sql: Sequence[str],
    statements: Sequence[str],
    syntax_only: bool,
    catalogue: bool = False,
) -> list[str]:
    """Pending referee files for `statements` in one environment; the case hashes."""
    environment: str = environment_sha256(session_parameters=PARAMETERS, setup_sql=setup_sql)
    (directory / "environments").mkdir(parents=True)
    (directory / "cases").mkdir()
    _write(
        directory / "environments" / f"{environment}.json",
        {
            "format": 1,
            "environment_sha256": environment,
            "session_parameters": PARAMETERS,
            "setup_sql": list(setup_sql),
        },
    )
    hashes: list[str] = []
    for index, statement in enumerate(statements):
        digest: str = case_sha256(
            source_dialect="snowflake",
            environment=environment,
            statement=statement,
            syntax_only=syntax_only,
        )
        hashes.append(digest)
        _write(
            directory / "cases" / f"{digest}.json",
            {
                "format": 1,
                "case_hash": digest,
                "case_reference": f"orders.json#{index:03}",
                "source_dialect": "snowflake",
                "environment_sha256": environment,
                "statement": statement,
                "syntax_only": syntax_only,
            },
        )
    for path in (directory / "catalogue.json",) * catalogue:
        _write(path, {"format": 1})
    return hashes


def record(*, pending: Path, evidence: Path) -> tuple[RefereeSummary, list[str]]:
    """Record the pending referee files with a fake session; the summary and session log."""
    loaded: RefereePending | None = load_referee(pending)
    assert loaded is not None
    log: list[str] = []

    def open_session(_: SessionParameters) -> RefereeSession:
        log.append("open")
        return FakeRefereeSession(log=log)

    summary: RefereeSummary = record_referee(
        pending=loaded, directory=evidence, open_session=open_session, clock=lambda: "2026-01-01"
    )
    return summary, log


def outcome(*, evidence: Path, digest: str) -> dict[str, object]:
    """The recorded outcome document for a case hash."""
    return json.loads((evidence / "results" / f"{digest}.json").read_text(encoding="utf-8"))


def outcome_kind(*, evidence: Path, digest: str) -> str:
    """The recorded outcome of a case hash, or `missing` when nothing was stored."""
    return next(
        (
            str(json.loads(path.read_text(encoding="utf-8"))["outcome"])
            for path in (evidence / "results").glob(f"{digest}.json")
        ),
        "missing",
    )


def catalogue_names(evidence: Path) -> list[list[str]]:
    """The function names of each stored catalogue."""
    return [_names(path) for path in sorted((evidence / "functions").glob("*.json"))]


def _names(path: Path) -> list[str]:
    functions: list[dict[str, str]] = json.loads(path.read_text(encoding="utf-8"))["functions"]
    return [function["name"] for function in functions]


def _write(path: Path, document: object) -> None:
    path.write_text(json.dumps(document), encoding="utf-8")


def scripted_pending(*, repository: Path, selection: str) -> RefereePending | None:
    """Generate the referee files, write pending with the repository script, and load it."""
    run_generate(["--repository", str(repository)])
    run_pending(["--repository", str(repository), "--select", selection])
    return load_referee(repository / "evidence" / "pending" / "referee")
