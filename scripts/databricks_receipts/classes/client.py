"""The SQL Statement Execution API, with OAuth M2M credentials from the environment."""

from __future__ import annotations

import base64
import json
import os
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

from scripts.databricks_receipts._helpers.rows import row_of
from scripts.databricks_receipts.constants import (
    CASE_SECONDS,
    CATALOG,
    ENV_CASE_SECONDS,
    ENV_CLIENT_ID,
    ENV_CLIENT_SECRET,
    ENV_HOST,
    ENV_WAREHOUSE,
    HTTP_ATTEMPTS,
    HTTP_PAUSE_SECONDS,
    HTTP_RETRY_STATUS,
    HTTP_SECONDS,
    HTTP_SERVER_ERROR,
    POLL_SECONDS,
    REPLY_LOST,
    STATE_FAILED,
    STATE_SUCCEEDED,
    STATEMENTS_PATH,
    STATES_PENDING,
    STATES_STOPPED,
    TOKEN_PATH,
    TOKEN_REFRESH_SECONDS,
    TOKEN_REJECTED,
    TOKEN_REJECTED_CODES,
    TOKEN_SCOPE,
    WAIT_TIMEOUT,
    WAREHOUSES_PATH,
)
from scripts.databricks_receipts.exceptions import (
    RefereeError,
    ReplyLost,
    TokenRejected,
)
from scripts.databricks_receipts.types import Row


def environment(name: str) -> str:
    """A required environment variable."""
    value: str | None = os.environ.get(name)
    if not value:
        raise RefereeError(f"{name} is not set")
    return value


class Client:
    """One warehouse, one service principal; a token refreshed before it expires; thread-safe."""

    def __init__(self) -> None:
        self.host: str = environment(ENV_HOST).rstrip("/")
        self.warehouse: str = environment(ENV_WAREHOUSE)
        self.case_seconds: float = float(os.environ.get(ENV_CASE_SECONDS) or CASE_SECONDS)
        self._lock: threading.Lock = threading.Lock()
        self._token: str = ""
        self._issued: float = 0.0

    def token(self) -> str:
        """A bearer token from the workspace's OAuth endpoint."""
        with self._lock:
            if self._token and time.time() - self._issued < TOKEN_REFRESH_SECONDS:
                return self._token
            pair: str = f"{environment(ENV_CLIENT_ID)}:{environment(ENV_CLIENT_SECRET)}"
            request: urllib.request.Request = urllib.request.Request(
                self.host + TOKEN_PATH,
                data=urllib.parse.urlencode(
                    {"grant_type": "client_credentials", "scope": TOKEN_SCOPE}
                ).encode(),
                headers={"Authorization": "Basic " + base64.b64encode(pair.encode()).decode()},
            )
            reply: Row = self._send(request=request)
            self._token, self._issued = str(reply["access_token"]), time.time()
            return self._token

    def _send(self, *, request: urllib.request.Request, once: bool = False) -> Row:
        """The JSON reply, retried after a 429, a 5xx or a transport failure unless sent `once`."""
        for attempt in range(HTTP_ATTEMPTS):
            try:
                with urllib.request.urlopen(request, timeout=HTTP_SECONDS) as reply:  # noqa: S310
                    sent: bytes = reply.read()
                    return json.loads(sent) if sent.strip() else {}
            except urllib.error.HTTPError as error:
                if (
                    error.code != HTTP_RETRY_STATUS
                    and error.code < HTTP_SERVER_ERROR
                    or attempt == HTTP_ATTEMPTS - 1
                ):
                    body: str = error.read().decode(errors="replace")[:500]
                    if error.code in TOKEN_REJECTED_CODES and TOKEN_REJECTED.search(body):
                        raise TokenRejected(
                            f"HTTP {error.code} from the warehouse API: {body}"
                        ) from error
                    raise RefereeError(
                        f"HTTP {error.code} from the warehouse API: {body}"
                    ) from error
            except (urllib.error.URLError, TimeoutError) as error:
                if once:
                    raise ReplyLost(f"{REPLY_LOST}: {error}") from error
                if attempt == HTTP_ATTEMPTS - 1:
                    raise RefereeError(f"the warehouse API is unreachable: {error}") from error
            time.sleep(HTTP_PAUSE_SECONDS * (attempt + 1))
        raise RefereeError("unreachable")

    def _call(self, *, method: str, path: str, body: Row | None = None, once: bool = False) -> Row:
        """The reply to one API request, sent again with a new token if its token is rejected."""
        try:
            return self._send(request=self._request(method=method, path=path, body=body), once=once)
        except TokenRejected:
            with self._lock:
                self._token = ""
            return self._send(request=self._request(method=method, path=path, body=body), once=once)

    def _request(self, *, method: str, path: str, body: Row | None) -> urllib.request.Request:
        return urllib.request.Request(
            self.host + path,
            method=method,
            data=None if body is None else json.dumps(body).encode(),
            headers={"Authorization": f"Bearer {self.token()}", "Content-Type": "application/json"},
        )

    def check(self) -> None:
        """Read the warehouse, so a token or a workspace that fails, fails before a case starts."""
        self._call(method="GET", path=f"{WAREHOUSES_PATH}/{self.warehouse}")

    def execute(self, *, sql: str, schema: str | None) -> Row:
        """The final status of one statement; one still running at the deadline is cancelled."""
        body: Row = {
            "warehouse_id": self.warehouse,
            "statement": sql,
            "catalog": CATALOG,
            "wait_timeout": WAIT_TIMEOUT,
            "on_wait_timeout": "CONTINUE",
            "disposition": "INLINE",
            "format": "JSON_ARRAY",
        }
        if schema is not None:
            body["schema"] = schema
        deadline: float = time.monotonic() + self.case_seconds
        reply: Row = self._call(method="POST", path=STATEMENTS_PATH, body=body, once=True)
        while str(row_of(reply["status"])["state"]) in STATES_PENDING:
            if time.monotonic() > deadline:
                identifier: str = str(reply["statement_id"])
                self._call(method="POST", path=f"{STATEMENTS_PATH}/{identifier}/cancel")
                raise RefereeError(
                    f"statement {identifier} was still running after {self.case_seconds:g} s; "
                    "cancelled, so its case is undecided"
                )
            time.sleep(POLL_SECONDS)
            reply = self._call(method="GET", path=f"{STATEMENTS_PATH}/{reply['statement_id']}")
        state: str = str(row_of(reply["status"])["state"])
        if state in STATES_STOPPED or state not in (STATE_SUCCEEDED, STATE_FAILED):
            raise RefereeError(f"statement ended {state}: {sql[:200]!r}")
        return reply

    def require(self, *, sql: str, schema: str | None) -> Row:
        """A statement that must succeed, such as setup or cleanup."""
        reply: Row = self.execute(sql=sql, schema=schema)
        status: Row = row_of(reply["status"])
        if status["state"] != STATE_SUCCEEDED:
            raise RefereeError(f"{sql[:200]!r} failed: {row_of(status.get('error') or {})}")
        return reply
