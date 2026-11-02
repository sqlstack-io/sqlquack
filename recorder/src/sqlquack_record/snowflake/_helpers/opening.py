"""Opening one Snowflake session with a warehouse in use."""

from __future__ import annotations

import snowflake.connector

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.types import SessionParameters
from sqlquack_record.snowflake._helpers.connections import connect_arguments
from sqlquack_record.snowflake.classes.snowflake_session import SnowflakeSession
from sqlquack_record.snowflake.models import ConnectionSettings


def open_snowflake_session(
    *, settings: ConnectionSettings, parameters: SessionParameters
) -> SnowflakeSession:
    """A session with the owner's connection settings and `parameters`; it needs a warehouse."""
    session: SnowflakeSession = SnowflakeSession(
        snowflake.connector.connect(**connect_arguments(settings=settings, parameters=parameters))
    )
    if session.current_warehouse() is None:
        session.close()
        raise RecorderError("no warehouse is in use; set one in the connection or with --warehouse")
    return session
