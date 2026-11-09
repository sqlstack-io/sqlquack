"""Connection settings and the clock the `sqlquack-record` command uses."""

from __future__ import annotations

import argparse
import os
from datetime import UTC, datetime

from sqlquack_record.recording.constants import CONNECTION_OVERRIDES
from sqlquack_record.snowflake.models import ConnectionSettings


def connection_settings(arguments: argparse.Namespace) -> ConnectionSettings:
    """The connection from --connection, --warehouse and the SNOWFLAKE_* overrides."""
    return ConnectionSettings(
        connection_name=arguments.connection,
        warehouse=arguments.warehouse,
        overrides=tuple(
            (argument, os.environ[variable])
            for variable, argument in sorted(CONNECTION_OVERRIDES.items())
            if os.environ.get(variable)
        ),
    )


def utc_now() -> str:
    """The current UTC time, RFC 3339 to the second."""
    return datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z")
