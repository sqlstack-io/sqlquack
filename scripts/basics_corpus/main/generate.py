"""Writes corpora/basics/queries.toml: every basic statement in several spellings, with outcomes."""

from __future__ import annotations

import argparse
import random
from pathlib import Path

from scripts.basics_corpus._helpers.outcomes import outcomes
from scripts.basics_corpus._helpers.spellings import respelled
from scripts.basics_corpus._helpers.statements import basics
from scripts.basics_corpus.constants import CORPUS_FILE, HEADER, REPOSITORY, SEED, SPELLINGS


def run_generate(argv: list[str] | None = None) -> int:
    """Generate the statements and record each one's outcome with the given `sqlquack` binary."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--record", type=Path, required=True, help="the sqlquack binary to record outcomes with"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    rng: random.Random = random.Random(SEED)
    blocks: list[str] = []
    for basic in basics():
        spellings: list[str] = [basic.sql] + [
            respelled(sql=basic.sql, rng=rng) for _ in range(SPELLINGS)
        ]
        for sql in dict.fromkeys(spellings):
            strict, repository = outcomes(binary=arguments.record, repository=REPOSITORY, sql=sql)
            blocks.append(
                f'[[query]]\ncategory = "{basic.shape}"\n'
                f'expect = "{strict}"\nexpect_repository = "{repository}"\n'
                f"sql = '''{sql}'''\n"
            )
    (REPOSITORY / CORPUS_FILE).write_text(HEADER + "\n" + "\n".join(blocks))
    return 0
