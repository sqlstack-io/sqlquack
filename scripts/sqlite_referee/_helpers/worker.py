"""The child process that judges one case for the referee, in a directory of its own."""

from __future__ import annotations

import os
import tempfile
from multiprocessing.connection import Connection
from pathlib import Path

from scripts.sqlite_referee._helpers.judge import observe
from scripts.sqlite_referee.classes.library import Library
from scripts.sqlite_referee.constants import (
    CHILD_DIR_PREFIX,
    ENGINE_VERSION,
    MESSAGE_DONE,
    MESSAGE_REFUSED,
    SOURCE_ID,
)
from scripts.sqlite_referee.exceptions import RefereeError
from scripts.sqlite_referee.models import Observation


def serve(
    *, channel: Connection, library_path: str, sql: str, setup: list[str], capture: str
) -> None:
    """The child: judge one case from an empty directory it removes, and send what it saw."""
    with tempfile.TemporaryDirectory(prefix=CHILD_DIR_PREFIX) as directory:
        os.chdir(directory)
        try:
            library: Library = Library(Path(library_path))
            if library.version() != (ENGINE_VERSION, SOURCE_ID):
                raise RefereeError(f"the library is {library.version()}, not the pinned release")
            observation: Observation = observe(
                library=library,
                sql=sql,
                setup=setup,
                capture=capture,
                report=lambda tag, value: channel.send((tag, value)),
            )
        except RefereeError as error:
            channel.send((MESSAGE_REFUSED, str(error)))
            return
        channel.send((MESSAGE_DONE, observation))
