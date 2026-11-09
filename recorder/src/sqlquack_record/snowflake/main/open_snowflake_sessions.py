"""Open Snowflake sessions with the owner's connection settings."""

from __future__ import annotations

from sqlquack_record.recording.types import SessionFactory, SessionParameters, WarehouseSession
from sqlquack_record.snowflake._helpers.opening import open_snowflake_session
from sqlquack_record.snowflake.models import ConnectionSettings


def snowflake_sessions(settings: ConnectionSettings) -> SessionFactory:
    """A factory opening one Snowflake session per set of session parameters."""

    def open_session(parameters: SessionParameters) -> WarehouseSession:
        return open_snowflake_session(settings=settings, parameters=parameters)

    return open_session
