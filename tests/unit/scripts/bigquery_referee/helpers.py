"""Recorded BigQuery API v2 replies, hand-written, and a transport that serves them."""

from __future__ import annotations

import re
from collections.abc import Callable

from scripts.bigquery_referee.types import Row

PROJECT: str = "test-project-1"
RUN_ID: str = "bigquery-x"
DATASET: str = "sqlquack_run_bigquery_x"


def _error(*, code: int, status: str, reason: str, message: str) -> Row:
    """An error reply in the documented shape: a google.rpc error with the legacy `errors` list."""
    return {
        "error": {
            "code": code,
            "message": message,
            "errors": [{"message": message, "domain": "global", "reason": reason}],
            "status": status,
        }
    }


SYNTAX_BODY: Row = _error(
    code=400,
    status="INVALID_ARGUMENT",
    reason="invalidQuery",
    message="Syntax error: Expected end of input but got keyword SELECT at [1:15]",
)
FOR_UPDATE_BODY: Row = _error(
    code=400,
    status="INVALID_ARGUMENT",
    reason="invalidQuery",
    message="FOR UPDATE is not supported at [1:10]",
)
FUNCTION_ARGUMENT_BODY: Row = _error(
    code=400,
    status="INVALID_ARGUMENT",
    reason="invalidQuery",
    message=(
        "Each function argument is an expression, not a query; to use a query as an expression, "
        "the query must be wrapped with additional parentheses to make it a scalar subquery "
        "expression at [1:12]"
    ),
)
NOT_FOUND_BODY: Row = _error(
    code=404,
    status="NOT_FOUND",
    reason="notFound",
    message=f"Not found: Table {PROJECT}:{DATASET}_2.t9 was not found in location US",
)
SIGNATURE_BODY: Row = _error(
    code=400,
    status="INVALID_ARGUMENT",
    reason="invalidQuery",
    message=(
        "No matching signature for function ABS for argument types: STRING. "
        "Supported signature: ABS(INT64) at [1:10]"
    ),
)
ACCESS_BODY: Row = _error(
    code=403,
    status="PERMISSION_DENIED",
    reason="accessDenied",
    message=(
        "Access Denied: Project other-project: User does not have bigquery.jobs.create "
        "permission in project other-project."
    ),
)
QUOTA_BODY: Row = _error(
    code=403,
    status="PERMISSION_DENIED",
    reason="quotaExceeded",
    message="Quota exceeded: Your project exceeded quota for free query bytes scanned.",
)
BILLING_BODY: Row = _error(
    code=403,
    status="PERMISSION_DENIED",
    reason="billingNotEnabled",
    message="Billing has not been enabled for this project. Enable billing at <url>.",
)
RATE_BODY: Row = _error(
    code=403,
    status="PERMISSION_DENIED",
    reason="rateLimitExceeded",
    message="Exceeded rate limits: too many concurrent queries for this project",
)
BACKEND_BODY: Row = _error(
    code=503,
    status="UNAVAILABLE",
    reason="backendError",
    message="Error encountered during execution.",
)
INTERNAL_BODY: Row = _error(
    code=500,
    status="INTERNAL",
    reason="internalError",
    message="An internal error occurred and the request could not be completed. Error: 80038528",
)
UNMAPPED_BODY: Row = _error(
    code=400,
    status="INVALID_ARGUMENT",
    reason="invalidQuery",
    message="Unexpected ALTER statement in a dry run at [1:1]",
)
CLEAN_BODY: Row = {
    "configuration": {"dryRun": True},
    "statistics": {"totalBytesProcessed": "0", "query": {"statementType": "SELECT"}},
    "status": {"state": "DONE"},
}
DONE_BODY: Row = {
    "configuration": {},
    "statistics": {"totalBytesProcessed": "0", "query": {"totalBytesBilled": "0"}},
    "status": {"state": "DONE"},
}
_MISSING: Row = _error(code=404, status="NOT_FOUND", reason="notFound", message="Not found")
_DATASET_BODY: Row = {"defaultTableExpirationMs": "3600000", "labels": {"sqlquack_run": DATASET}}

type DryReply = Callable[[str], tuple[int, Row]]


def clean(_: str) -> tuple[int, Row]:
    """A dry run that raises nothing."""
    return 200, CLEAN_BODY


def refuse_unmapped(sql: str) -> tuple[int, Row]:
    """A dry run that raises an unmapped error for an ALTER text and nothing for others."""
    return {True: (400, UNMAPPED_BODY), False: (200, CLEAN_BODY)}["ALTER" in sql]


def internal_on_select_2(sql: str) -> tuple[int, Row]:
    """A dry run that gives internalError for a `SELECT 2` text and nothing for others."""
    return {True: (500, INTERNAL_BODY), False: (200, CLEAN_BODY)}["SELECT 2" in sql]


def deny(_: str) -> tuple[int, Row]:
    """A dry run refused for permission."""
    return 403, ACCESS_BODY


def replies_of(replies: list[tuple[int, Row]]) -> Callable[..., tuple[int, Row]]:
    """A transport that gives the replies in order, whatever it is asked."""
    return lambda **_: replies.pop(0)


class RecordedBigQuery:
    """Serves recorded replies by route; datasets exist from their creation to their drop."""

    def __init__(self, *, dry: DryReply = clean) -> None:
        self.dry: DryReply = dry
        self.calls: list[tuple[str, str]] = []
        self.bodies: list[Row] = []
        self.created: list[str] = []
        self.dropped: list[str] = []
        self.jobs: list[Row] = []
        self.executed: list[str] = []

    def __call__(self, *, method: str, path: str, body: Row | None) -> tuple[int, Row]:
        sent: Row = body or {}
        self.calls.append((method, path))
        self.bodies.append(sent)
        bare: str = path.split("?")[0].removeprefix(f"/projects/{PROJECT}")
        route: str = re.sub(
            r"/datasets/\w+", "/datasets/<d>", re.sub(r"/jobs/\w+", "/jobs/<j>", bare)
        )
        dry: bool = bool(dict(sent.get("configuration") or {}).get("dryRun"))
        dataset: str = "".join(re.findall(r"/datasets/(\w+)", bare))
        return self._routes()[(method, route, dry)](sent, dataset)

    def _routes(self) -> dict[tuple[str, str, bool], Callable[[Row, str], tuple[int, Row]]]:
        return {
            ("POST", "/jobs", True): self._dry_job,
            ("POST", "/jobs", False): self._run_job,
            ("GET", "/jobs/<j>", False): lambda *_: (200, DONE_BODY),
            ("POST", "/datasets", False): self._create,
            ("DELETE", "/datasets/<d>", False): self._drop,
            ("GET", "/datasets/<d>", False): self._dataset,
            ("GET", "/datasets/<d>/tables", False): lambda *_: (200, {}),
            ("GET", "/datasets/<d>/routines", False): lambda *_: (200, {}),
            ("GET", "/datasets/<d>/models", False): lambda *_: (200, {}),
        }

    def _dry_job(self, sent: Row, _: str) -> tuple[int, Row]:
        self.jobs.append(sent)
        return self.dry(str(sent["configuration"]["query"]["query"]))

    def _run_job(self, sent: Row, _: str) -> tuple[int, Row]:
        self.jobs.append(sent)
        self.executed.append(str(sent["configuration"]["query"]["query"]))
        return 200, {"status": {"state": "RUNNING"}}

    def _create(self, sent: Row, _: str) -> tuple[int, Row]:
        self.created.append(str(sent["datasetReference"]["datasetId"]))
        return 200, sent

    def _drop(self, _: Row, dataset: str) -> tuple[int, Row]:
        self.dropped.append(dataset)
        return 204, {}

    def _dataset(self, _: Row, dataset: str) -> tuple[int, Row]:
        return {True: (404, _MISSING), False: (200, _DATASET_BODY)}[dataset in self.dropped]

    def touched(self) -> set[str]:
        """Every dataset any call named."""
        return set(re.findall(r"/datasets/(\w+)", " ".join(path for _, path in self.calls)))

    def left(self) -> set[str]:
        """Datasets created and not dropped."""
        return set(self.created) - set(self.dropped)


def case_row(*, case_id: str, sql: str) -> Row:
    """A case as the case files hold it, with no setup or session."""
    return {
        "capture": "execute_autocommit_in_throwaway_database",
        "case_id": case_id,
        "expect": "accept",
        "session": {},
        "session_sha256": "0" * 64,
        "setup": [],
        "setup_sha256": "0" * 64,
        "sql": sql,
        "sql_sha256": "0" * 64,
    }


def no_sleep(_: float) -> None:
    """Backoff that does not wait."""


class Sleeps:
    """Backoff that records each wait and does not wait."""

    def __init__(self) -> None:
        self.waits: list[float] = []

    def __call__(self, seconds: float) -> None:
        self.waits.append(seconds)


def refuse_from_from(sql: str) -> tuple[int, Row]:
    """A dry run that raises a syntax error for a text with `FROM FROM` and nothing for others."""
    return {True: (400, SYNTAX_BODY), False: (200, CLEAN_BODY)}["FROM FROM" in sql]


def run_cases() -> list[Row]:
    """A small run: a query, a syntax error, a script, an insert and a LOAD DATA."""
    return [
        case_row(case_id="bq.query.a", sql="SELECT 1 FROM SQLQUACK_DATASET.t1"),
        case_row(case_id="bq.query.b", sql="SELECT 1 FROM FROM SQLQUACK_DATASET.t2"),
        case_row(case_id="bq.utility.c", sql="BEGIN SELECT 1 ; END"),
        case_row(case_id="bq.dml.d", sql="INSERT SQLQUACK_DATASET.t1 VALUES ( 1 )"),
        case_row(case_id="bq.dml.e", sql="LOAD DATA INTO SQLQUACK_DATASET.t1 FROM FILES ( )"),
    ]
