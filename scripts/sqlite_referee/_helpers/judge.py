"""Judge one case in a fresh in-memory SQLite database: prepare it alone, then run it."""

from __future__ import annotations

import ctypes
from collections.abc import Callable
from functools import partial

from scripts.sqlite_referee.classes.library import Library
from scripts.sqlite_referee.constants import (
    ACCEPTED,
    BASIS_EMPTY,
    BASIS_PARSE,
    BASIS_PARSE_LATE,
    BASIS_PREPARE,
    BASIS_RAN,
    BASIS_RAN_AUTOCOMMIT,
    BASIS_STEP,
    BEGIN,
    CAPTURE_AUTOCOMMIT,
    MESSAGE_FORMAT,
    MESSAGE_PARSED,
    MESSAGE_SET_UP,
    PRIMARY_MASK,
    ROLLBACK,
    RUNTIME_ERROR,
    RUNTIME_MESSAGES,
    SEMANTIC_CODE,
    SEMANTIC_ERROR,
    SEMANTIC_MESSAGES,
    SQLITE_ERROR,
    SQLITE_OK,
    SYNTAX_ERROR,
    SYNTAX_MESSAGES,
    TEXT_ONLY_CODE,
    TEXT_ONLY_MESSAGES,
    UNSUPPORTED_CODE,
    UNSUPPORTED_FEATURE,
    UNSUPPORTED_MESSAGES,
)
from scripts.sqlite_referee.exceptions import RefereeError
from scripts.sqlite_referee.models import Observation, Prepared, StepResult


def syntax_code(*, message: str, statement: str, alone: Callable[[], str | None]) -> str | None:
    """The syntax class of a prepare error the parser raised from the text alone, else None."""
    for pattern, code in SYNTAX_MESSAGES:
        if pattern.fullmatch(message):
            return code
    for pattern, gate in TEXT_ONLY_MESSAGES:
        if (
            pattern.fullmatch(message)
            and (gate is None or gate.match(statement))
            and alone() == message
        ):
            return TEXT_ONLY_CODE
    return None


def prepare_phase(
    *, code: int, message: str, statement: str, alone: Callable[[], str | None]
) -> tuple[str, str]:
    """The engine code and phase of a prepare error; one in no reviewed class stops the run."""
    found: str | None = syntax_code(message=message, statement=statement, alone=alone)
    if found is not None:
        return found, SYNTAX_ERROR
    if any(pattern.fullmatch(message) for pattern in UNSUPPORTED_MESSAGES):
        return UNSUPPORTED_CODE, UNSUPPORTED_FEATURE
    if code & PRIMARY_MASK != SQLITE_ERROR or any(
        pattern.fullmatch(message) for pattern in RUNTIME_MESSAGES
    ):
        return SEMANTIC_CODE.format(code=code), RUNTIME_ERROR
    if any(pattern.fullmatch(message) for pattern in SEMANTIC_MESSAGES) or any(
        pattern.fullmatch(message) for pattern, _ in TEXT_ONLY_MESSAGES
    ):
        return SEMANTIC_CODE.format(code=code), SEMANTIC_ERROR
    raise RefereeError(f"prepare error in no reviewed class (code {code}): {message}")


def alone_message(*, library: Library, sql: bytes, offset: int) -> str | None:
    """The prepare error of the statement at `offset`, prepared alone in an empty database."""
    handle: ctypes.c_void_p = library.open()
    try:
        prepared: Prepared = library.prepare(handle=handle, text=sql, offset=offset)
        if prepared.statement is not None:
            library.lib.sqlite3_finalize(prepared.statement)
            return None
        return prepared.message
    finally:
        library.close(handle)


def message_of(*, name: str, code: int, message: str) -> str:
    """The result code's name, the extended code and sqlite3_errmsg(), verbatim."""
    return MESSAGE_FORMAT.format(name=name, code=code, message=message)


def run_plain(*, library: Library, handle: ctypes.c_void_p, text: str) -> None:
    """Run one setup or control statement; any error stops the case as a refused run."""
    prepared: Prepared = library.prepare(handle=handle, text=text.encode(), offset=0)
    if prepared.statement is None:
        raise RefereeError(f"statement {text!r} failed to prepare: {prepared.message}")
    stepped: StepResult = library.step_all(handle=handle, statement=prepared.statement)
    if stepped.code != SQLITE_OK:
        raise RefereeError(f"statement {text!r} failed: {stepped.message}")


def connected(*, library: Library, setup: list[str]) -> ctypes.c_void_p:
    """A fresh in-memory database with the case's setup applied in autocommit."""
    handle: ctypes.c_void_p = library.open()
    try:
        for text in setup:
            run_plain(library=library, handle=handle, text=text)
    except RefereeError:
        library.close(handle)
        raise
    return handle


def parse_only(
    *, library: Library, handle: ctypes.c_void_p, sql: bytes
) -> tuple[Observation | None, int | None]:
    """Prepare each statement unrun: the parser's refusal or None, and the count if all prepared."""
    offset: int = 0
    index: int = 0
    while offset < len(sql):
        prepared: Prepared = library.prepare(handle=handle, text=sql, offset=offset)
        if prepared.statement is None and prepared.code == SQLITE_OK:
            if prepared.end <= offset:
                break
            offset = prepared.end
            continue
        index += 1
        if prepared.statement is None:
            found: str | None = syntax_code(
                message=prepared.message,
                statement=sql[offset:].decode(errors="replace"),
                alone=partial(alone_message, library=library, sql=sql, offset=offset),
            )
            if found is None:
                return None, None
            return Observation(
                phase=SYNTAX_ERROR,
                basis=BASIS_PARSE.format(index=index, code=found),
                engine_code=found,
                parse_only_code=found,
                message=message_of(
                    name=prepared.name, code=prepared.code, message=prepared.message
                ),
                statements=index,
            ), None
        library.lib.sqlite3_finalize(prepared.statement)
        offset = prepared.end
    return None, index


def observe(
    *,
    library: Library,
    sql: str,
    setup: list[str],
    capture: str,
    report: Callable[[str, int], None],
) -> Observation:
    """The case's phase: the parse-only pass, then each statement run in its capture."""
    text: bytes = sql.encode()
    autocommit: bool = capture == CAPTURE_AUTOCOMMIT
    checking: ctypes.c_void_p = connected(library=library, setup=setup)
    handle: ctypes.c_void_p = connected(library=library, setup=setup)
    try:
        if not autocommit:
            run_plain(library=library, handle=handle, text=BEGIN)
        report(MESSAGE_SET_UP, 0)
        refused, count = parse_only(library=library, handle=checking, sql=text)
        if refused is not None:
            return refused
        if count is not None:
            report(MESSAGE_PARSED, count)
        observation: Observation = _execute(
            library=library,
            handle=handle,
            sql=text,
            ran=BASIS_RAN_AUTOCOMMIT if autocommit else BASIS_RAN,
        )
        if library.in_transaction(handle):
            run_plain(library=library, handle=handle, text=ROLLBACK)
        return observation
    finally:
        library.close(checking)
        library.close(handle)


def _execute(*, library: Library, handle: ctypes.c_void_p, sql: bytes, ran: str) -> Observation:
    """Prepare and step each statement in turn; the first error decides the phase."""
    offset: int = 0
    index: int = 0
    while offset < len(sql):
        prepared: Prepared = library.prepare(handle=handle, text=sql, offset=offset)
        if prepared.statement is None and prepared.code == SQLITE_OK:
            if prepared.end <= offset:
                break
            offset = prepared.end
            continue
        index += 1
        if prepared.statement is None:
            code, phase = prepare_phase(
                code=prepared.code,
                message=prepared.message,
                statement=sql[offset:].decode(errors="replace"),
                alone=partial(alone_message, library=library, sql=sql, offset=offset),
            )
            basis: str = (BASIS_PARSE_LATE if phase == SYNTAX_ERROR else BASIS_PREPARE).format(
                index=index, code=code
            )
            return Observation(
                phase=phase,
                basis=basis,
                engine_code=code,
                parse_only_code=None,
                message=message_of(
                    name=prepared.name, code=prepared.code, message=prepared.message
                ),
                statements=index,
            )
        stepped: StepResult = library.step_all(handle=handle, statement=prepared.statement)
        if stepped.code != SQLITE_OK:
            step_code: str = SEMANTIC_CODE.format(code=stepped.code)
            return Observation(
                phase=RUNTIME_ERROR,
                basis=BASIS_STEP.format(index=index, code=step_code),
                engine_code=step_code,
                parse_only_code=None,
                message=message_of(name=stepped.name, code=stepped.code, message=stepped.message),
                statements=index,
            )
        offset = prepared.end
    return Observation(
        phase=ACCEPTED,
        basis=ran if index else BASIS_EMPTY,
        engine_code=None,
        parse_only_code=None,
        message=None,
        statements=index,
    )
