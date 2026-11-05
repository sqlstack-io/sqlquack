"""The native module the ``sqlquack`` package wraps; use the package's functions instead."""

__version__: str

def check_json(sql: str, path: str) -> str:
    """``{"report": <check --json report>, "rendered": <text>}``; ``ValueError`` without SQL."""

def convert_json(sql: str, request: str, path: str) -> str:
    """``{"report": <convert --json report>, "rendered": <text>}``; ``ValueError`` on exit 2."""
