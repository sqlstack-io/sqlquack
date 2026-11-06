"""Writes corpora/basics/queries.toml with unrecorded outcomes for `make corpus-rerecord`."""

from __future__ import annotations

import argparse
import random

from scripts.basics_corpus._helpers.spellings import respelled
from scripts.basics_corpus._helpers.statements import basics
from scripts.basics_corpus.constants import (
    CORPUS_FILE,
    HEADER,
    REPOSITORY,
    SEED,
    SPELLINGS,
    UNRECORDED,
)


def run_generate(argv: list[str] | None = None) -> int:
    """Generate the statements, each with an unrecorded outcome."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    rng: random.Random = random.Random(SEED)
    blocks: list[str] = []
    for basic in basics():
        spellings: list[str] = [basic.sql] + [
            respelled(sql=basic.sql, rng=rng) for _ in range(SPELLINGS)
        ]
        for sql in dict.fromkeys(spellings):
            blocks.append(
                f'[[query]]\ncategory = "{basic.shape}"\n'
                f"sql = '''{sql}'''\n"
                f'expect = "{UNRECORDED}"\nexpect_repository = "{UNRECORDED}"\n'
            )
    (REPOSITORY / CORPUS_FILE).write_text(HEADER + "\n" + "\n".join(blocks))
    return 0
