"""Open Snowflake sessions for referee environments."""

from __future__ import annotations

from sqlquack_record.recording.types import SessionParameters
from sqlquack_record.referee.types import RefereeSession, RefereeSessionFactory
from sqlquack_record.snowflake._helpers.opening import open_snowflake_session
from sqlquack_record.snowflake.models import ConnectionSettings


def referee_sessions(settings: ConnectionSettings) -> RefereeSessionFactory:
    """A factory opening one Snowflake session per referee environment."""

    def open_session(parameters: SessionParameters) -> RefereeSession:
        return open_snowflake_session(settings=settings, parameters=parameters)

    return open_session
