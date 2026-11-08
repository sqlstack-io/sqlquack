"""Judge each case in its own child process, stopped when the case outlives its time budget."""

from __future__ import annotations

import multiprocessing
import time
from multiprocessing.connection import Connection
from multiprocessing.context import ForkServerContext, ForkServerProcess
from pathlib import Path

from scripts.sqlite_referee._helpers.worker import serve
from scripts.sqlite_referee.constants import (
    BASIS_TIMEOUT,
    MESSAGE_DONE,
    MESSAGE_PARSED,
    MESSAGE_REFUSED,
    MESSAGE_SET_UP,
    MESSAGE_TIMEOUT,
    RUNTIME_ERROR,
    STAGE_EXECUTE,
    STAGE_PARSE,
    START_METHOD,
    TIMEOUT_CODE,
    TIMEOUT_SECONDS,
)
from scripts.sqlite_referee.exceptions import RefereeError
from scripts.sqlite_referee.models import Observation


class Referee:
    """A fresh child process per case, each within `seconds` from setup to the last step."""

    def __init__(self, *, library: Path, seconds: float = TIMEOUT_SECONDS) -> None:
        """Ready to judge; `seconds` covers setup, the parse-only pass and execution."""
        self.library: Path = library
        self.seconds: float = seconds
        self.context: ForkServerContext = multiprocessing.get_context(START_METHOD)

    def observe(self, *, case: str, sql: str, setup: list[str], capture: str) -> Observation:
        """The case's observation; a case that outlives the budget has its child stopped."""
        mine, theirs = self.context.Pipe()
        process: ForkServerProcess = self.context.Process(
            target=serve,
            kwargs={
                "channel": theirs,
                "library_path": str(self.library),
                "sql": sql,
                "setup": setup,
                "capture": capture,
            },
            daemon=True,
        )
        process.start()
        theirs.close()
        try:
            return self._wait(case=case, channel=mine)
        finally:
            process.kill()
            process.join()
            mine.close()

    def _wait(self, *, case: str, channel: Connection) -> Observation:
        """Read the child's messages until it is done or the one deadline passes."""
        deadline: float = time.monotonic() + self.seconds
        stage: str = STAGE_PARSE
        statements: int | None = None
        while channel.poll(max(0.0, deadline - time.monotonic())):
            try:
                tag, value = channel.recv()
            except (EOFError, OSError) as error:
                raise RefereeError(f"case {case}: the child process died during {stage}") from error
            if tag == MESSAGE_SET_UP:
                stage = STAGE_EXECUTE
            elif tag == MESSAGE_PARSED:
                statements = int(value)
            elif tag == MESSAGE_DONE:
                return value
            else:
                reason: str = str(value) if tag == MESSAGE_REFUSED else f"unknown reply {tag}"
                raise RefereeError(f"case {case}: {reason}")
        if statements is None:
            raise RefereeError(
                f"case {case}: the {self.seconds:g} s budget ran out during {stage} before every "
                "statement had prepared, so its syntax is undecided"
            )
        return Observation(
            phase=RUNTIME_ERROR,
            basis=BASIS_TIMEOUT.format(seconds=self.seconds),
            engine_code=TIMEOUT_CODE,
            parse_only_code=None,
            message=MESSAGE_TIMEOUT.format(seconds=self.seconds),
            statements=statements,
        )
