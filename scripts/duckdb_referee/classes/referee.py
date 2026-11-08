"""Judge cases in a child process that is stopped when a case outlives its time budget."""

from __future__ import annotations

import multiprocessing
import time
from multiprocessing.connection import Connection
from multiprocessing.context import SpawnContext, SpawnProcess
from types import TracebackType

from scripts.duckdb_referee._helpers.worker import serve
from scripts.duckdb_referee.constants import (
    BASIS_TIMEOUT,
    MESSAGE_DONE,
    MESSAGE_PARSED,
    MESSAGE_REFUSED,
    MESSAGE_SET_UP,
    MESSAGE_TIMEOUT,
    RUNTIME_ERROR,
    STAGE_PARSE,
    STAGE_SETUP,
    START_METHOD,
    TIMEOUT_CODE,
    TIMEOUT_SECONDS,
)
from scripts.duckdb_referee.exceptions import RefereeError
from scripts.duckdb_referee.models import Observation


class Referee:
    """A child process judging one case at a time, each within `seconds` from setup to EXPLAIN."""

    def __init__(self, seconds: float = TIMEOUT_SECONDS) -> None:
        """Ready to start the child; `seconds` covers setup, parsing, execution and EXPLAIN."""
        self.seconds: float = seconds
        self.context: SpawnContext = multiprocessing.get_context(START_METHOD)
        self.channel: Connection | None = None
        self.process: SpawnProcess | None = None

    def __enter__(self) -> Referee:
        """Start the child."""
        self._start()
        return self

    def __exit__(
        self,
        kind: type[BaseException] | None,
        error: BaseException | None,
        trace: TracebackType | None,
    ) -> None:
        """Stop the child."""
        self._stop()

    def _start(self) -> None:
        mine, theirs = self.context.Pipe()
        self.process = self.context.Process(target=serve, args=(theirs,), daemon=True)
        self.process.start()
        theirs.close()
        self.channel = mine

    def _stop(self) -> None:
        if self.process is not None:
            self.process.kill()
            self.process.join()
        if self.channel is not None:
            self.channel.close()
        self.process, self.channel = None, None

    def observe(
        self, *, case: str, sql: str, session: dict[str, str], setup: list[str]
    ) -> Observation:
        """The case's observation; a case that outlives the budget stops the child."""
        if self.channel is None:
            raise RefereeError(f"case {case}: the referee's child process is not running")
        self.channel.send((sql, session, setup))
        deadline: float = time.monotonic() + self.seconds
        stage: str = STAGE_SETUP
        statements: int | None = None
        while self.channel.poll(max(0.0, deadline - time.monotonic())):
            try:
                tag, value = self.channel.recv()
            except (EOFError, OSError) as error:
                self._stop()
                self._start()
                raise RefereeError(f"case {case}: the child process died during {stage}") from error
            if tag == MESSAGE_SET_UP:
                stage = STAGE_PARSE
            elif tag == MESSAGE_PARSED:
                statements = int(value)
            elif tag == MESSAGE_DONE:
                return value
            else:
                reason: str = str(value) if tag == MESSAGE_REFUSED else f"unknown reply {tag}"
                raise RefereeError(f"case {case}: {reason}")
        self._stop()
        self._start()
        if statements is None:
            raise RefereeError(
                f"case {case}: the {self.seconds:g} s budget ran out during {stage}, so its "
                "syntax is undecided"
            )
        return Observation(
            phase=RUNTIME_ERROR,
            basis=BASIS_TIMEOUT.format(seconds=self.seconds),
            engine_code=TIMEOUT_CODE,
            parse_only_code=None,
            message=MESSAGE_TIMEOUT.format(seconds=self.seconds),
            statements=statements,
        )
