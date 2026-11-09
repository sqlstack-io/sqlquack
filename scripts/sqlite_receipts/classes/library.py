"""A ctypes binding of the few C interfaces the referee and the catalogue fetch call."""

from __future__ import annotations

import ctypes
from pathlib import Path

from scripts.sqlite_receipts.constants import (
    OPEN_FLAGS,
    SQLITE_DONE,
    SQLITE_OK,
    SQLITE_ROW,
)
from scripts.sqlite_receipts.exceptions import RefereeError
from scripts.sqlite_receipts.models import Prepared, StepResult


class Library:
    """The pinned SQLite library, loaded from one shared object."""

    def __init__(self, path: Path) -> None:
        """Load the library and declare the signatures of the interfaces used."""
        self.path: Path = path
        lib: ctypes.CDLL = ctypes.CDLL(str(path))
        pointer: type[ctypes.c_void_p] = ctypes.c_void_p
        lib.sqlite3_libversion.restype = ctypes.c_char_p
        lib.sqlite3_sourceid.restype = ctypes.c_char_p
        lib.sqlite3_open_v2.argtypes = [
            ctypes.c_char_p,
            ctypes.POINTER(pointer),
            ctypes.c_int,
            ctypes.c_char_p,
        ]
        lib.sqlite3_close_v2.argtypes = [pointer]
        lib.sqlite3_prepare_v2.argtypes = [
            pointer,
            ctypes.c_void_p,
            ctypes.c_int,
            ctypes.POINTER(pointer),
            ctypes.POINTER(pointer),
        ]
        lib.sqlite3_step.argtypes = [pointer]
        lib.sqlite3_finalize.argtypes = [pointer]
        lib.sqlite3_errmsg.argtypes = [pointer]
        lib.sqlite3_errmsg.restype = ctypes.c_char_p
        lib.sqlite3_errstr.argtypes = [ctypes.c_int]
        lib.sqlite3_errstr.restype = ctypes.c_char_p
        lib.sqlite3_extended_errcode.argtypes = [pointer]
        lib.sqlite3_get_autocommit.argtypes = [pointer]
        lib.sqlite3_column_count.argtypes = [pointer]
        lib.sqlite3_column_text.argtypes = [pointer, ctypes.c_int]
        lib.sqlite3_column_text.restype = ctypes.c_char_p
        lib.sqlite3_keyword_count.argtypes = []
        lib.sqlite3_keyword_name.argtypes = [
            ctypes.c_int,
            ctypes.POINTER(ctypes.c_char_p),
            ctypes.POINTER(ctypes.c_int),
        ]
        self.lib: ctypes.CDLL = lib

    def version(self) -> tuple[str, str]:
        """sqlite3_libversion() and sqlite3_sourceid()."""
        return (
            self.lib.sqlite3_libversion().decode(),
            self.lib.sqlite3_sourceid().decode(),
        )

    def keywords(self) -> list[str]:
        """Every keyword sqlite3_keyword_name() reports, in its order."""
        found: list[str] = []
        for index in range(self.lib.sqlite3_keyword_count()):
            name: ctypes.c_char_p = ctypes.c_char_p()
            size: ctypes.c_int = ctypes.c_int()
            self.lib.sqlite3_keyword_name(index, ctypes.byref(name), ctypes.byref(size))
            found.append(ctypes.string_at(name, size.value).decode())
        return found

    def open(self) -> ctypes.c_void_p:
        """A fresh in-memory database connection."""
        handle: ctypes.c_void_p = ctypes.c_void_p()
        rc: int = self.lib.sqlite3_open_v2(b":memory:", ctypes.byref(handle), OPEN_FLAGS, None)
        if rc != SQLITE_OK:
            raise RefereeError(f"sqlite3_open_v2 failed with {rc}")
        return handle

    def close(self, handle: ctypes.c_void_p) -> None:
        """Close a connection, finalizing nothing: every statement is finalized when used."""
        self.lib.sqlite3_close_v2(handle)

    def error(self, *, handle: ctypes.c_void_p, rc: int) -> tuple[int, str, str]:
        """The extended result code, its name text and the connection's error message."""
        extended: int = self.lib.sqlite3_extended_errcode(handle)
        message: bytes = self.lib.sqlite3_errmsg(handle) or b""
        name: bytes = self.lib.sqlite3_errstr(extended or rc) or b""
        return extended or rc, name.decode(errors="replace"), message.decode(errors="replace")

    def prepare(self, *, handle: ctypes.c_void_p, text: bytes, offset: int) -> Prepared:
        """sqlite3_prepare_v2 of the text from `offset`: the statement, the tail and any error."""
        buffer: ctypes.Array[ctypes.c_char] = ctypes.create_string_buffer(text)
        start: int = ctypes.addressof(buffer) + offset
        statement: ctypes.c_void_p = ctypes.c_void_p()
        tail: ctypes.c_void_p = ctypes.c_void_p()
        rc: int = self.lib.sqlite3_prepare_v2(
            handle, start, len(text) - offset, ctypes.byref(statement), ctypes.byref(tail)
        )
        end: int = (tail.value - ctypes.addressof(buffer)) if tail.value else len(text)
        if rc != SQLITE_OK:
            code, name, message = self.error(handle=handle, rc=rc)
            self.lib.sqlite3_finalize(statement)
            return Prepared(statement=None, end=end, code=code, name=name, message=message)
        return Prepared(statement=statement, end=end, code=SQLITE_OK, name="", message="")

    def step_all(self, *, handle: ctypes.c_void_p, statement: ctypes.c_void_p) -> StepResult:
        """Step a prepared statement to completion and finalize it, reading no value."""
        while True:
            rc: int = self.lib.sqlite3_step(statement)
            if rc == SQLITE_ROW:
                continue
            if rc == SQLITE_DONE:
                self.lib.sqlite3_finalize(statement)
                return StepResult(code=SQLITE_OK, name="", message="")
            code, name, message = self.error(handle=handle, rc=rc)
            self.lib.sqlite3_finalize(statement)
            return StepResult(code=code, name=name, message=message)

    def rows(self, *, handle: ctypes.c_void_p, sql: str) -> list[list[str | None]]:
        """Every row of one statement, its values as text; any error raises."""
        prepared: Prepared = self.prepare(handle=handle, text=sql.encode(), offset=0)
        if prepared.statement is None:
            raise RefereeError(f"{sql}: {prepared.message}")
        found: list[list[str | None]] = []
        columns: int = self.lib.sqlite3_column_count(prepared.statement)
        while (rc := self.lib.sqlite3_step(prepared.statement)) == SQLITE_ROW:
            values: list[str | None] = []
            for column in range(columns):
                value: bytes | None = self.lib.sqlite3_column_text(prepared.statement, column)
                values.append(None if value is None else value.decode())
            found.append(values)
        self.lib.sqlite3_finalize(prepared.statement)
        if rc != SQLITE_DONE:
            raise RefereeError(f"{sql}: step returned {rc}")
        return found

    def in_transaction(self, handle: ctypes.c_void_p) -> bool:
        """Whether the connection is inside an explicit transaction."""
        return self.lib.sqlite3_get_autocommit(handle) == 0
