"""Judge one case in a schema of its own: create it, set it up, run the case, drop it."""

from __future__ import annotations

import re

from scripts.databricks_referee._helpers.rows import row_of
from scripts.databricks_referee.classes.client import Client
from scripts.databricks_referee.constants import (
    ACCEPTED,
    ADDRESS,
    ADDRESS_MASK,
    BASIS_LATER,
    BASIS_PARSER,
    BASIS_RAN,
    CASE_SCHEMA,
    CASE_SCHEMA_MASK,
    CATALOG,
    DROP_ATTEMPTS,
    ENVIRONMENT_MASKS,
    ERROR_CLASS,
    EXCERPT_RUNTIME_CLASSES,
    EXCERPT_UNSUPPORTED_CLASSES,
    LATER_PHASES,
    LATER_WITHOUT_SQLSTATE,
    PARSER_CLASSES,
    PARSER_MARK,
    PARSER_MESSAGE_CODE,
    REQUEST_ID,
    REQUEST_ID_MASK,
    RUNTIME_ERROR,
    SERVICE_ERROR_CLASS,
    SQLSTATE_OF,
    STATE_SUCCEEDED,
    SYNTAX_ERROR,
    UNCLASSED,
    UNCLASSED_LATER,
    UNCLASSED_PARSER,
    UNSUPPORTED_FEATURE,
    URL,
    URL_MASK,
)
from scripts.databricks_referee.exceptions import RefereeError
from scripts.databricks_referee.models import Failure, Observation
from scripts.databricks_referee.types import Row


def failure_of(reply: Row) -> Failure:
    """The error class (or UNCLASSED), SQLSTATE and message of a failed statement."""
    status: Row = row_of(reply["status"])
    error: Row = row_of(status.get("error") or {})
    message: str = str(error.get("message", ""))
    if not message.strip():
        raise RefereeError(f"a failed statement without a message: {status}")
    found: re.Match[str] | None = ERROR_CLASS.match(message) or SERVICE_ERROR_CLASS.match(message)
    sqlstate: str = str(status.get("sql_state") or "")
    stated: re.Match[str] | None = SQLSTATE_OF.search(message)
    if not sqlstate and stated:
        sqlstate = stated.group(1)
    return Failure(
        error_class=found.group(1) if found else UNCLASSED,
        sqlstate=sqlstate,
        message=message,
        parser=PARSER_MARK.search(message) is not None,
        statement_id=str(reply.get("statement_id", "")),
    )


def phase_of(failure: Failure) -> str | None:
    """The phase of a failure, or None when nothing reviewed decides it, which stops the run."""
    if failure.error_class == UNCLASSED:
        message: str = failure.message.lstrip()
        if failure.parser:
            matched: bool = any(pattern.match(message) for pattern in UNCLASSED_PARSER)
            return SYNTAX_ERROR if matched else None
        for pattern, phase in UNCLASSED_LATER:
            if pattern.match(message):
                return phase
        return None
    if (
        not failure.sqlstate
        and not SERVICE_ERROR_CLASS.match(failure.message)
        and (failure.parser or failure.error_class not in LATER_WITHOUT_SQLSTATE)
    ):
        return None
    if failure.parser:
        if failure.error_class in PARSER_CLASSES:
            return SYNTAX_ERROR
        if failure.error_class in EXCERPT_RUNTIME_CLASSES:
            return RUNTIME_ERROR
        return UNSUPPORTED_FEATURE if failure.error_class in EXCERPT_UNSUPPORTED_CLASSES else None
    return LATER_PHASES.get(failure.error_class)


def published(message: str) -> str:
    """The message as a receipt carries it: its first line, with what is not public masked."""
    first: str = message.strip().splitlines()[0] if message.strip() else ""
    masked: str = REQUEST_ID.sub(
        REQUEST_ID_MASK, ADDRESS.sub(ADDRESS_MASK, URL.sub(URL_MASK, first))
    )
    for pattern, mask in ENVIRONMENT_MASKS:
        masked = pattern.sub(mask, masked)
    return CASE_SCHEMA.sub(CASE_SCHEMA_MASK, masked)


def observation_of(reply: Row) -> Observation | Failure:
    """The observation of a case's reply, or the failure whose class has no reviewed phase."""
    statement: str = str(reply.get("statement_id", ""))
    if row_of(reply["status"])["state"] == STATE_SUCCEEDED:
        return Observation(
            phase=ACCEPTED,
            basis=BASIS_RAN,
            engine_code=None,
            parse_only_code=None,
            message=None,
            error_class=None,
            statement_id=statement,
        )
    return observation_from(failure_of(reply))


def observation_from(failure: Failure) -> Observation | Failure:
    """The observation a failure's reviewed phase gives, or the failure when none decides it."""
    phase: str | None = phase_of(failure)
    if phase is None:
        return failure
    return Observation(
        phase=phase,
        basis=(BASIS_PARSER if phase == SYNTAX_ERROR else BASIS_LATER).format(
            error_class=failure.error_class
        ),
        engine_code=failure.sqlstate
        or (PARSER_MESSAGE_CODE if failure.parser and phase == SYNTAX_ERROR else None),
        parse_only_code=(failure.sqlstate or None) if phase == SYNTAX_ERROR else None,
        message=published(failure.message),
        error_class=failure.error_class,
        statement_id=failure.statement_id,
    )


def observe(*, client: Client, schema: str, sql: str, setup: list[str]) -> Observation | Failure:
    """The case run in a new schema holding only its setup; the schema is dropped afterwards."""
    try:
        client.require(sql=f"CREATE SCHEMA {CATALOG}.{schema}", schema=None)
        for text in setup:
            client.require(sql=text, schema=schema)
        return observation_of(client.execute(sql=sql, schema=schema))
    finally:
        drop(client=client, schema=schema)


def drop(*, client: Client, schema: str) -> None:
    """Drop the case's schema, asking again after a failure, since the drop is idempotent."""
    for attempt in range(DROP_ATTEMPTS):
        try:
            client.require(sql=f"DROP SCHEMA IF EXISTS {CATALOG}.{schema} CASCADE", schema=None)
            return
        except RefereeError:
            if attempt == DROP_ATTEMPTS - 1:
                raise
