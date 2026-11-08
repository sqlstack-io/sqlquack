"""The child process that judges cases one at a time for the referee."""

from __future__ import annotations

from multiprocessing.connection import Connection

from scripts.duckdb_referee._helpers.judge import observe
from scripts.duckdb_referee.constants import (
    MESSAGE_DONE,
    MESSAGE_PARSED,
    MESSAGE_REFUSED,
    MESSAGE_SET_UP,
)
from scripts.duckdb_referee.exceptions import RefereeError
from scripts.duckdb_referee.models import Observation


def serve(channel: Connection) -> None:
    """The child: judge each case it receives and send back what it saw, step by step."""
    while True:
        sql, session, setup = channel.recv()
        try:
            observation: Observation = observe(
                sql=sql,
                session=session,
                setup=setup,
                set_up=lambda: channel.send((MESSAGE_SET_UP, 0)),
                parsed=lambda count: channel.send((MESSAGE_PARSED, count)),
            )
        except RefereeError as error:
            channel.send((MESSAGE_REFUSED, str(error)))
            continue
        channel.send((MESSAGE_DONE, observation))
