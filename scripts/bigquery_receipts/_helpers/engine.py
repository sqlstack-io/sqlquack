"""The authenticated transport to the BigQuery API v2, from a service-account key."""

from __future__ import annotations

import functools
import json
import os
import threading
from pathlib import Path

from scripts.bigquery_receipts.constants import (
    API_ROOT,
    HTTP_TIMEOUT_SECONDS,
    JSON_OPEN,
    KEY_VARIABLE,
    SCOPE,
)
from scripts.bigquery_receipts.exceptions import RefereeError
from scripts.bigquery_receipts.types import Row, Transport


def _key_info() -> dict[str, object]:
    """The key from the variable, which holds the JSON itself or the path of its file."""
    value: str = os.environ.get(KEY_VARIABLE, "")
    if not value.strip():
        raise RefereeError(f"set {KEY_VARIABLE} to the service-account key JSON or its path")
    try:
        text: str = value if value.lstrip().startswith(JSON_OPEN) else Path(value).read_text()
        info: object = json.loads(text)
    except (OSError, ValueError):
        raise RefereeError(
            f"{KEY_VARIABLE} holds neither key JSON nor a readable key file"
        ) from None
    if not isinstance(info, dict):
        raise RefereeError(f"{KEY_VARIABLE} does not hold a key object")
    return info


def environment_transport() -> Transport:
    """A transport that sends authorised requests, one session per worker thread."""
    from google.auth.exceptions import TransportError
    from google.auth.transport.requests import AuthorizedSession
    from google.oauth2 import service_account
    from requests import RequestException, Response

    credentials: service_account.Credentials = (
        service_account.Credentials.from_service_account_info(_key_info(), scopes=[SCOPE])
    )

    @functools.cache
    def session_of(thread: int) -> AuthorizedSession:
        return AuthorizedSession(credentials)

    def send(*, method: str, path: str, body: Row | None) -> tuple[int, Row]:
        session: AuthorizedSession = session_of(threading.get_ident())
        try:
            response: Response = session.request(
                method, API_ROOT + path, json=body, timeout=HTTP_TIMEOUT_SECONDS
            )
        except (RequestException, TransportError):
            return 0, {}
        try:
            reply: object = response.json() if response.content else {}
        except ValueError:
            reply = {}
        return int(response.status_code), reply if isinstance(reply, dict) else {}

    return send
