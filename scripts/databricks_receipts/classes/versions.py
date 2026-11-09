"""The warehouse release that answers a run's cases."""

from __future__ import annotations

import threading
import time

from scripts.databricks_receipts._helpers.receipts import canonical, version_of
from scripts.databricks_receipts.classes.client import Client
from scripts.databricks_receipts.constants import VERSION_QUERY, VERSION_SECONDS
from scripts.databricks_receipts.types import Row


class Versions:
    """current_version(), asked again once VERSION_SECONDS have passed; every answer is kept."""

    def __init__(self, client: Client) -> None:
        self.client: Client = client
        self.lock: threading.Lock = threading.Lock()
        self.current: Row = {}
        self.asked: float = 0.0
        self.seen: set[str] = set()

    def ask(self) -> Row:
        """The release now answering, as current_version() reports it."""
        version: Row = version_of(self.client.require(sql=VERSION_QUERY, schema=None))
        self.seen.add(canonical(version))
        return version

    def get(self) -> Row:
        with self.lock:
            if not self.current or time.monotonic() - self.asked > VERSION_SECONDS:
                self.current, self.asked = self.ask(), time.monotonic()
            return self.current
