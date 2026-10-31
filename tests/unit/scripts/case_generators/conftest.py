"""Fixtures shared by the case generator tests."""

from __future__ import annotations

import pytest

from scripts.case_generators._helpers.registry import generated_texts


@pytest.fixture(scope="session")
def first_generation() -> dict[str, str]:
    """Every generated file's text, generated once for the session."""
    return generated_texts()


@pytest.fixture(scope="session")
def second_generation() -> dict[str, str]:
    """Every generated file's text from a second, independent generation."""
    return generated_texts()
