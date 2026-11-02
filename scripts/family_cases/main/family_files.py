"""Family case grids: zoned timestamps from text, and VARIANT order, equality and numbers."""

from __future__ import annotations

from scripts.case_generators.models import CaseFile
from scripts.family_cases._helpers.variant_order import variant_order_files
from scripts.family_cases._helpers.zoned_texts import zoned_text_files


def family_files() -> list[CaseFile]:
    """Every family's files: zoned texts first, then VARIANT order."""
    return [*zoned_text_files(), *variant_order_files()]
