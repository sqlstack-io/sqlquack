"""The omission audit fails a form with a usable grammar and no obligations, and lists the forms
that have none by design with the reason."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.syntax_obligations_snowflake._helpers.audit import omission_audit
from scripts.syntax_obligations_snowflake.constants import (
    AUDIT_ALIAS_TARGETS_EMPTY,
    AUDIT_ALIASED,
    AUDIT_DERIVED_NONE,
    AUDIT_EMPTY_GRAMMAR,
    AUDIT_FRAGMENT,
    AUDIT_PROSE_UNREAD,
    AUDIT_SUPPRESSED,
    AUDIT_WITHDRAWN,
)
from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import (
    AuditBuildTestCase,
    OmissionAuditTestCase,
)
from tests.unit.scripts.syntax_obligations_snowflake.helpers import FAMILY, catalogue_form

USE_ROLE: str = "sf.transaction_session.use-role"
OLD_USE: str = "sf.transaction_session.use-old"


USE_ROLE_FORM: str = catalogue_form(identifier=USE_ROLE, grammars='["USE ROLE <name>"]')
RETIRED_FORM: str = catalogue_form(identifier=OLD_USE, grammars="[]", status="absent_from_source")
ALIAS: str = f'[[alias]]\nfrom = "{OLD_USE}"\nto = ["{USE_ROLE}"]\n'


@pytest.mark.parametrize(
    "test_case",
    (
        OmissionAuditTestCase(
            description="a form with obligations is neither failed nor listed",
            catalogue=USE_ROLE_FORM,
            aliases="",
            limits="",
            built=(USE_ROLE,),
            dropped=(),
            expected_failures=(),
            expected_listed=(),
        ),
        OmissionAuditTestCase(
            description="a grammar form without obligations fails",
            catalogue=USE_ROLE_FORM,
            aliases="",
            limits="",
            built=(),
            dropped=(),
            expected_failures=(f"{USE_ROLE}: {AUDIT_DERIVED_NONE}",),
            expected_listed=(),
        ),
        OmissionAuditTestCase(
            description="a form whose obligations were all withdrawn fails with that reason",
            catalogue=USE_ROLE_FORM,
            aliases="",
            limits="",
            built=(),
            dropped=((USE_ROLE, AUDIT_WITHDRAWN),),
            expected_failures=(f"{USE_ROLE}: {AUDIT_WITHDRAWN}",),
            expected_listed=(),
        ),
        OmissionAuditTestCase(
            description="a form whose obligations limits.toml suppresses fails with that reason",
            catalogue=USE_ROLE_FORM,
            aliases="",
            limits=f'[[suppress]]\nform = "{USE_ROLE}"\nkind = "positive"\nreason = "r"\n',
            built=(),
            dropped=(),
            expected_failures=(f"{USE_ROLE}: {AUDIT_SUPPRESSED}",),
            expected_listed=(),
        ),
        OmissionAuditTestCase(
            description="a catalogued form with an empty grammar and no reading fails",
            catalogue=catalogue_form(identifier=USE_ROLE, grammars="[]"),
            aliases="",
            limits="",
            built=(),
            dropped=(),
            expected_failures=(f"{USE_ROLE}: {AUDIT_EMPTY_GRAMMAR}",),
            expected_listed=(),
        ),
        OmissionAuditTestCase(
            description="a prose form with no reading is listed",
            catalogue=catalogue_form(identifier=USE_ROLE, grammars="[]", kind="prose"),
            aliases="",
            limits="",
            built=(),
            dropped=(),
            expected_failures=(),
            expected_listed=(f"{USE_ROLE}: {AUDIT_PROSE_UNREAD}",),
        ),
        OmissionAuditTestCase(
            description="a retired fragment without an alias is listed",
            catalogue=RETIRED_FORM,
            aliases="",
            limits="",
            built=(),
            dropped=(),
            expected_failures=(),
            expected_listed=(f"{OLD_USE}: {AUDIT_FRAGMENT}",),
        ),
        OmissionAuditTestCase(
            description="a retired form whose alias carries obligations is listed",
            catalogue=RETIRED_FORM + USE_ROLE_FORM,
            aliases=ALIAS,
            limits="",
            built=(USE_ROLE,),
            dropped=(),
            expected_failures=(),
            expected_listed=(f"{OLD_USE}: {AUDIT_ALIASED} {USE_ROLE}",),
        ),
        OmissionAuditTestCase(
            description="a retired form whose alias carries no obligations fails",
            catalogue=RETIRED_FORM,
            aliases=ALIAS,
            limits="",
            built=(),
            dropped=(),
            expected_failures=(f"{OLD_USE}: {AUDIT_ALIAS_TARGETS_EMPTY}: {USE_ROLE}",),
            expected_listed=(),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_form_without_obligations_when_auditing_then_it_fails_or_is_listed_with_why(
    tmp_path: Path, test_case: OmissionAuditTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / f"{FAMILY}.toml").write_text(test_case.catalogue)
    (tmp_path / "aliases.toml").write_text(test_case.aliases)
    (tmp_path / "limits.toml").write_text(test_case.limits)

    failures, listed = omission_audit(
        root=tmp_path, built={FAMILY: set(test_case.built)}, dropped=dict(test_case.dropped)
    )

    assert (tuple(failures), tuple(listed)) == (
        test_case.expected_failures,
        test_case.expected_listed,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        AuditBuildTestCase(description="writing the obligations", arguments=(), expected_code=0),
        AuditBuildTestCase(
            description="checking the obligations", arguments=("--check",), expected_code=0
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_retired_fragment_when_building_then_the_build_passes_and_lists_it(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], test_case: AuditBuildTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / f"{FAMILY}.toml").write_text(USE_ROLE_FORM + RETIRED_FORM)
    (tmp_path / "limits.toml").write_text("")
    assert run_build(["--root", str(tmp_path), FAMILY]) == 0
    capsys.readouterr()

    code: int = run_build(["--root", str(tmp_path), *test_case.arguments, FAMILY])

    assert (code, f"{OLD_USE}: {AUDIT_FRAGMENT}" in capsys.readouterr().out) == (
        test_case.expected_code,
        True,
    )
