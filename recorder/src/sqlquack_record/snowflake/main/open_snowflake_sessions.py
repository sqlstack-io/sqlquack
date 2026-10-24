"""Open Snowflake sessions with the owner's connection settings."""

from __future__ import annotations

from typing import Any

import snowflake.connector

from sqlquack_record.constants import QUERY_TAG
from sqlquack_record.errors import RecorderError
from sqlquack_record.models import SessionValue
from sqlquack_record.snowflake._helpers.session import SnowflakeSession
from sqlquack_record.snowflake.models import ConnectionSettings
from sqlquack_record.types import SessionFactory, WarehouseSession


def snowflake_sessions(settings: ConnectionSettings) -> SessionFactory:
    """A factory opening one Snowflake session per set of session parameters."""

    def open_session(parameters: tuple[tuple[str, SessionValue], ...]) -> WarehouseSession:
        arguments: dict[str, Any] = {
            "session_parameters": {**dict(parameters), "QUERY_TAG": QUERY_TAG},
            "arrow_number_to_decimal": True,
            "client_store_temporary_credential": True,
            "oauth_enable_refresh_tokens": True,
            **dict(settings.overrides),
        }
        if settings.connection_name is not None:
            arguments["connection_name"] = settings.connection_name
        if settings.warehouse is not None:
            arguments["warehouse"] = settings.warehouse
        session: SnowflakeSession = SnowflakeSession(snowflake.connector.connect(**arguments))
        if session.current_warehouse() is None:
            session.close()
            raise RecorderError(
                "no warehouse is in use; set one in the connection or with --warehouse"
            )
        return session

    return open_session
