"""The replay a change needs: none, the cases whose judgement inputs changed, or every case."""

from __future__ import annotations

from typing import Literal

type Scope = Literal["none", "changed", "full"]
