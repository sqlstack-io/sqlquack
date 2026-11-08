"""The pinned GoogleSQL parser as a pre-filter: accept cases parse, reject cases do not."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from scripts.bigquery_referee._helpers.refusals import syntax_classes
from scripts.bigquery_referee.models import ErrorClass
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_bigquery.constants import (
    AGREEMENT_SHOWN,
    ENGINE_CODE_KEY,
    EXPECT_ACCEPT,
    EXPECT_OBSERVE,
    EXPECT_REJECT,
    FIXTURES,
    OBJECT_TYPE_REFUSAL,
    PARSER_FLAGS,
    PARSER_SHA256,
    PARSER_STDIN,
    PARSER_TOKENS,
    PARSER_WORKERS,
    SHAPE_CODE,
    STAGE_KEY,
    SYNTAX_PHASE,
    TOKEN_PATTERN,
)
from scripts.syntax_obligations_bigquery.models import EngineEvidence, Verdict


def parser_text(sql: str) -> str:
    """The case text with each SQLQUACK_* token replaced by the plain name the parser reads."""
    return re.sub(TOKEN_PATTERN, lambda match: PARSER_TOKENS[match.group(0)], sql)


def check_parser(path: Path) -> None:
    """Refuse a parser binary that is not the pinned release asset."""
    if not path.is_file():
        raise ObligationError(f"no parser at {path}")
    if hashlib.sha256(path.read_bytes()).hexdigest() != PARSER_SHA256:
        raise ObligationError(f"{path} is not the pinned execute_query_linux ({PARSER_SHA256})")


def _parse(*, parser: Path, sql: str, workdir: str) -> Verdict:
    """One text through the parser: parsed, or the first line of its refusal."""
    done: subprocess.CompletedProcess[str] = subprocess.run(
        [str(parser), *PARSER_FLAGS, PARSER_STDIN],
        input=parser_text(sql),
        capture_output=True,
        text=True,
        cwd=workdir,
        check=False,
    )
    if done.returncode == 0:
        return Verdict(parsed=True, message="")
    said: str = (done.stdout + done.stderr).strip()
    return Verdict(parsed=False, message=said.split("\n", 1)[0])


def verdicts(*, parser: Path, texts: list[str]) -> dict[str, Verdict]:
    """Each distinct text's verdict, run from an empty directory on a worker pool."""
    check_parser(parser)
    distinct: list[str] = sorted(set(texts))
    with tempfile.TemporaryDirectory() as workdir:
        with ThreadPoolExecutor(max_workers=os.cpu_count() or PARSER_WORKERS) as pool:
            found: list[Verdict] = list(
                pool.map(lambda sql: _parse(parser=parser, sql=sql, workdir=workdir), distinct)
            )
    return dict(zip(distinct, found, strict=True))


def excepted(verdict: Verdict) -> bool:
    """Whether a refusal is the open build's missing object type, the one reviewed exception."""
    return re.match(OBJECT_TYPE_REFUSAL, verdict.message) is not None


def must_parse(case: Row) -> bool:
    """Whether the parser must take a case: an accept, or a reject expected after parsing."""
    return case["expect"] == EXPECT_ACCEPT or bool(case.get(STAGE_KEY))


def engine_confirmed(*, cases: list[Row], phases: dict[tuple[str, str], str]) -> frozenset[str]:
    """The cases whose own receipt gives their expectation: the engine, not the proxy, decides."""
    return frozenset(
        str(case["case_id"])
        for case in cases
        if (phase := phases.get((str(case["case_id"]), str(case["sql_sha256"]))))
        and (phase == SYNTAX_PHASE) == (case["expect"] == EXPECT_REJECT and not must_parse(case))
        and case["expect"] in (EXPECT_ACCEPT, EXPECT_REJECT)
    )


def analysis_shaped(directory: Path) -> frozenset[str]:
    """The case texts' hashes whose receipt the referee classed SHAPE, an analysis restriction."""
    shaped: set[str] = set()
    for path in sorted(directory.glob("*.jsonl")):
        receipts: list[Row] = [json.loads(line) for line in path.read_text().splitlines()]
        shaped |= {str(r["sql_sha256"]) for r in receipts if r.get(ENGINE_CODE_KEY) == SHAPE_CODE}
    return frozenset(shaped)


def refuse_unparsed_shapes(
    *, cases: list[Row], found: dict[str, Verdict], shaped: frozenset[str]
) -> None:
    """Stop the build when a SHAPE receipt's text does not parse: SHAPE is analysis, not syntax."""
    refused: list[str] = [
        f"{case['case_id']}: {case['sql']}"
        for case in cases
        if str(case["sql_sha256"]) in shaped
        and not found[str(case["sql"])].parsed
        and not excepted(found[str(case["sql"])])
    ]
    if refused:
        shown: str = "\n".join(refused[:AGREEMENT_SHOWN])
        raise ObligationError(
            f"{len(refused)} SHAPE receipts the pinned parser refuses; the class took a syntax "
            f"error:\n{shown}"
        )


def disagreements(
    *, cases: list[Row], found: dict[str, Verdict], confirmed: frozenset[str] = frozenset()
) -> list[str]:
    """Each case the parser takes but must refuse, or refuses but must take; less the exception."""
    wrong: list[str] = []
    for case in cases:
        verdict: Verdict = found[str(case["sql"])]
        if case["expect"] not in (EXPECT_ACCEPT, EXPECT_REJECT) or case["case_id"] in confirmed:
            continue
        if must_parse(case) and not verdict.parsed and not excepted(verdict):
            wrong.append(f"{case['case_id']} must parse, refused: {verdict.message}: {case['sql']}")
        if not must_parse(case) and verdict.parsed:
            wrong.append(f"{case['case_id']} must not parse, parses: {case['sql']}")
    return wrong


def agreement(
    *, cases: list[Row], found: dict[str, Verdict], confirmed: frozenset[str] = frozenset()
) -> Row:
    """The figures the README reports: cases checked, those under the exception, disagreements."""
    gated: list[Row] = [c for c in cases if c["expect"] in (EXPECT_ACCEPT, EXPECT_REJECT)]
    return {
        "cases": len(cases),
        "checked": len(gated),
        "must_parse": sum(must_parse(c) for c in gated),
        "object_type_exception": sum(
            must_parse(c) and excepted(found[str(c["sql"])]) for c in gated
        ),
        "engine_over_parser": len(disagreements(cases=gated, found=found))
        - len(disagreements(cases=gated, found=found, confirmed=confirmed)),
        "disagreements": len(disagreements(cases=gated, found=found, confirmed=confirmed)),
    }


def unmapped_refusals(*, cases: list[Row], found: dict[str, Verdict]) -> list[str]:
    """Each distinct refusal of a case that no syntax class of the referee maps."""
    classes: tuple[ErrorClass, ...] = syntax_classes()
    said: set[str] = {
        found[str(c["sql"])].message for c in cases if not found[str(c["sql"])].parsed
    }
    return sorted(filter(lambda message: not _mapped(message=message, classes=classes), said))


def _mapped(*, message: str, classes: tuple[ErrorClass, ...]) -> bool:
    """Whether some syntax class matches a refusal message."""
    return any(re.search(found.pattern, message) for found in classes)


def refuse_unmapped_refusals(*, cases: list[Row], found: dict[str, Verdict]) -> None:
    """Stop the build on a parser refusal the referee would not read as a syntax error."""
    unmapped: list[str] = unmapped_refusals(cases=cases, found=found)
    if unmapped:
        shown: str = "\n".join(unmapped[:AGREEMENT_SHOWN])
        raise ObligationError(f"parser refusals no referee syntax class maps:\n{shown}")


def refuse_unparsed_fixtures(found: dict[str, Verdict]) -> None:
    """Stop the build when a declared fixture's DDL does not parse."""
    refused: list[str] = [f.name for f in FIXTURES if not found[f.ddl].parsed]
    if refused:
        raise ObligationError(f"fixtures the pinned parser refuses: {refused}")


def refuse_disagreements(
    *, cases: list[Row], found: dict[str, Verdict], confirmed: frozenset[str]
) -> None:
    """Stop the build on any disagreement no reading, suppression, limit or receipt explains."""
    wrong: list[str] = disagreements(cases=cases, found=found, confirmed=confirmed)
    if wrong:
        shown: str = "\n".join(wrong[:AGREEMENT_SHOWN])
        raise ObligationError(
            f"{len(wrong)} cases the pinned parser contradicts; fix the root cause or add a "
            f"reviewed reading, suppression or limit:\n{shown}"
        )


def settle_observed(*, cases: list[Row], settle: frozenset[str], found: dict[str, Verdict]) -> None:
    """A suppressed observe text the parser refuses, not for its object type, becomes a reject."""
    for case in cases:
        if case["expect"] == EXPECT_OBSERVE and str(case["sql"]) in settle:
            verdict: Verdict = found[str(case["sql"])]
            if not verdict.parsed and not excepted(verdict):
                case["expect"] = EXPECT_REJECT


def refuse_refused_withdrawals(
    *, withdrawn: list[tuple[str, str]], found: dict[str, Verdict]
) -> None:
    """Stop the build when a `documented` suppression withdraws a text the parser refuses."""
    refused: list[str] = [f"{form}: {sql}" for form, sql in withdrawn if not found[sql].parsed]
    if refused:
        shown: str = "\n".join(refused[:AGREEMENT_SHOWN])
        raise ObligationError(
            f"{len(refused)} suppressed texts the pinned parser refuses, so each suppression "
            f"withdraws a valid negative; make it `observe`:\n{shown}"
        )


def gate(
    *,
    parser: Path,
    cases: list[Row],
    settle: frozenset[str] = frozenset(),
    withdrawn: list[tuple[str, str]] | None = None,
    evidence: EngineEvidence | None = None,
) -> dict[str, Verdict]:
    """Each case, suppressed text and fixture through the pinned parser; refuse contradictions."""
    taken: list[tuple[str, str]] = withdrawn or []
    settled: EngineEvidence = evidence or EngineEvidence()
    found: dict[str, Verdict] = verdicts(
        parser=parser,
        texts=[
            *(str(case["sql"]) for case in cases),
            *(sql for _, sql in taken),
            *(f.ddl for f in FIXTURES),
        ],
    )
    refuse_unparsed_fixtures(found)
    refuse_refused_withdrawals(withdrawn=taken, found=found)
    settle_observed(cases=cases, settle=settle, found=found)
    refuse_disagreements(cases=cases, found=found, confirmed=settled.confirmed)
    refuse_unparsed_shapes(cases=cases, found=found, shaped=settled.shaped)
    refuse_unmapped_refusals(cases=cases, found=found)
    return found
