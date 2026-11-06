"""`udf`: marks a project's Python function, as SQLBuild's `sqlbuild.functions.udf` does."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any


def udf(**kwargs: object) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    """Mark a Python function as a project function; its keywords are read statically."""
    del kwargs

    def _decorate(function: Callable[..., Any]) -> Callable[..., Any]:
        return function

    return _decorate
