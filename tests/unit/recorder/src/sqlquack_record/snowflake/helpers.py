"""Connection arguments against a fake connector configuration."""

from __future__ import annotations

from typing import Any

import pytest

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.snowflake._helpers import connections
from sqlquack_record.snowflake._helpers.connections import connect_arguments
from sqlquack_record.snowflake.models import ConnectionSettings

FAKE_CONFIGURATION: dict[str, Any] = {
    "default_connection_name": "shop",
    "connections": {
        "shop": {
            "account": "example-account",
            "authenticator": "oauth_authorization_code",
            "warehouse": "SMALL",
            "session_parameters": {"TIMEZONE": "America/New_York", "QUERY_TAG": "other"},
        },
        "explicit": {"account": "example-account", "client_store_temporary_credential": False},
    },
}


def arguments_for(
    *,
    monkeypatch: pytest.MonkeyPatch,
    connection_name: str | None,
    warehouse: str | None,
    overrides: tuple[tuple[str, str], ...],
) -> dict[str, object] | None:
    """The connect arguments for case parameters `TIMEZONE=UTC`, or `None` when refused."""
    monkeypatch.setattr(connections, "CONFIG_MANAGER", FAKE_CONFIGURATION)
    settings: ConnectionSettings = ConnectionSettings(
        connection_name=connection_name, warehouse=warehouse, overrides=overrides
    )
    try:
        return connect_arguments(settings=settings, parameters=(("TIMEZONE", "UTC"),))
    except RecorderError:
        return None
