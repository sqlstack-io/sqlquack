"""Open Snowflake sessions with the owner's connection settings."""

from __future__ import annotations

import snowflake.connector

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.types import SessionFactory, SessionParameters, WarehouseSession
from sqlquack_record.snowflake._helpers.connections import connect_arguments
from sqlquack_record.snowflake.classes.snowflake_session import SnowflakeSession
from sqlquack_record.snowflake.models import ConnectionSettings


def snowflake_sessions(settings: ConnectionSettings) -> SessionFactory:
    """A factory opening one Snowflake session per set of session parameters."""

    def open_session(parameters: SessionParameters) -> WarehouseSession:
        session: SnowflakeSession = SnowflakeSession(
            snowflake.connector.connect(
                **connect_arguments(settings=settings, parameters=parameters)
            )
        )
        if session.current_warehouse() is None:
            session.close()
            raise RecorderError(
                "no warehouse is in use; set one in the connection or with --warehouse"
            )
        return session

    return open_session
