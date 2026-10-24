"""Connection arguments from the connector's configuration and environment overrides."""

from __future__ import annotations

import pytest

from tests.unit.recorder.src.sqlquack_record.snowflake._test_types import (
    ConnectArgumentsTestCase,
)
from tests.unit.recorder.src.sqlquack_record.snowflake.helpers import arguments_for

_CASE_SESSION: dict[str, object] = {"TIMEZONE": "UTC", "QUERY_TAG": "sqlquack-record"}


@pytest.mark.parametrize(
    "test_case",
    (
        ConnectArgumentsTestCase(
            description="the default connection, with token caching and the case's parameters",
            connection_name=None,
            warehouse=None,
            overrides=(),
            expected_arguments={
                "account": "example-account",
                "authenticator": "oauth_authorization_code",
                "warehouse": "SMALL",
                "client_store_temporary_credential": True,
                "oauth_enable_refresh_tokens": True,
                "session_parameters": _CASE_SESSION,
                "arrow_number_to_decimal": True,
            },
        ),
        ConnectArgumentsTestCase(
            description="a named connection keeps its explicit settings; --warehouse wins",
            connection_name="explicit",
            warehouse="OTHER",
            overrides=(),
            expected_arguments={
                "account": "example-account",
                "warehouse": "OTHER",
                "client_store_temporary_credential": False,
                "oauth_enable_refresh_tokens": True,
                "session_parameters": _CASE_SESSION,
                "arrow_number_to_decimal": True,
            },
        ),
        ConnectArgumentsTestCase(
            description="an account from the environment replaces the configured connection",
            connection_name=None,
            warehouse="SMALL",
            overrides=(("account", "env-account"), ("user", "someone")),
            expected_arguments={
                "account": "env-account",
                "user": "someone",
                "warehouse": "SMALL",
                "client_store_temporary_credential": True,
                "oauth_enable_refresh_tokens": True,
                "session_parameters": _CASE_SESSION,
                "arrow_number_to_decimal": True,
            },
        ),
        ConnectArgumentsTestCase(
            description="an unknown connection name is refused",
            connection_name="missing",
            warehouse=None,
            overrides=(),
            expected_arguments=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_connection_settings_when_connecting_then_arguments_merge_in_order(
    test_case: ConnectArgumentsTestCase, monkeypatch: pytest.MonkeyPatch
) -> None:
    arguments: dict[str, object] | None = arguments_for(
        monkeypatch=monkeypatch,
        connection_name=test_case.connection_name,
        warehouse=test_case.warehouse,
        overrides=test_case.overrides,
    )

    assert arguments == test_case.expected_arguments
