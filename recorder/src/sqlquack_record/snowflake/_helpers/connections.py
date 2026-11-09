"""Resolving connection arguments from the connector's own configuration."""

from __future__ import annotations

from typing import Any

from snowflake.connector.config_manager import CONFIG_MANAGER

from sqlquack_record.recording.constants import (
    ACCOUNT_ARGUMENT,
    ARROW_RESULT_FORMAT,
    CREDENTIAL_CACHE_DEFAULTS,
    QUERY_TAG,
    RESULT_FORMAT_PARAMETER,
    SESSION_PARAMETERS_ARGUMENT,
    WAREHOUSE_ARGUMENT,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.types import SessionParameters
from sqlquack_record.snowflake.models import ConnectionSettings


def connect_arguments(
    *, settings: ConnectionSettings, parameters: SessionParameters
) -> dict[str, Any]:
    """Named or default connection, then cache defaults, overrides and the case's parameters."""
    overrides: dict[str, str] = dict(settings.overrides)
    named: dict[str, Any] = {} if ACCOUNT_ARGUMENT in overrides else _connection(settings)
    arguments: dict[str, Any] = {**CREDENTIAL_CACHE_DEFAULTS, **named, **overrides}
    arguments[SESSION_PARAMETERS_ARGUMENT] = {
        **dict(named.get(SESSION_PARAMETERS_ARGUMENT) or {}),
        **dict(parameters),
        "QUERY_TAG": QUERY_TAG,
        RESULT_FORMAT_PARAMETER: ARROW_RESULT_FORMAT,
    }
    if settings.warehouse is not None:
        arguments[WAREHOUSE_ARGUMENT] = settings.warehouse
    arguments["arrow_number_to_decimal"] = True
    return arguments


def _connection(settings: ConnectionSettings) -> dict[str, Any]:
    connections: Any = CONFIG_MANAGER["connections"]
    name: str = settings.connection_name or str(CONFIG_MANAGER["default_connection_name"])
    if name not in connections:
        raise RecorderError(
            f"no Snowflake connection named '{name}' in the connector configuration; pass "
            "--connection or set SNOWFLAKE_ACCOUNT and related variables"
        )
    return dict(connections[name])
