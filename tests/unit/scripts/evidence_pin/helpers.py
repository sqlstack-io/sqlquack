"""Throwaway evidence repositories, scripts/evidence.sh run against them, and each case."""

from __future__ import annotations

import os
import subprocess
import tomllib
from pathlib import Path

import pytest

from scripts.evidence_pin._helpers.content_hash import content_sha256
from scripts.evidence_pin.constants import EVIDENCE_SCRIPT, PIN_FILE, PLACEHOLDER_COMMIT
from scripts.evidence_pin.main.check import run_pin_check
from tests.unit.scripts.evidence_pin._test_types import (
    Change,
    EvidenceRepositories,
    ScriptOutcome,
)

PIN_HEADER: str = (
    "# The evidence SQLQuack's checks read: one commit of the evidence repository, "
    "and the SHA-256 of its files.\n"
    'repository = "https://github.com/sqlstack-io/sqlquack-evidence.git"\n'
)
_PLACEHOLDER_HASH: str = "0" * 64
_PLACEHOLDER_PIN: str = (
    f'{PIN_HEADER}commit = "{PLACEHOLDER_COMMIT}"\ncontent_sha256 = "{_PLACEHOLDER_HASH}"\n'
)
_STORED_RECEIPT: str = '{"case_id": "query.stored"}\n'
_STORED_RECEIPTS_FILE: str = "syntax/receipts/stored.jsonl"
_LISTING_PIPELINE: str = "find . -type f -printf '%P\\0' | LC_ALL=C sort -z | xargs -0 sha256sum"
_SCRIPT_SWITCHES: tuple[str, ...] = ("EVIDENCE", "REFRESH")
_READ_ONLY_BITS: int = 0o222


def git(repository: Path, *arguments: str) -> str:
    """Run Git in `repository` as a test identity, and return its trimmed output."""
    return subprocess.run(
        ("git", "-c", "user.name=Test", "-c", "user.email=test@example.com", *arguments),
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def evidence_repositories(tmp_path: Path) -> EvidenceRepositories:
    """A bare evidence remote whose main holds one commit, and a SQLQuack checkout pinning none."""
    repositories: EvidenceRepositories = EvidenceRepositories(
        remote=tmp_path / "remote.git",
        author=tmp_path / "author",
        root=tmp_path / "sqlquack",
        clone=tmp_path / "clone",
    )
    git(tmp_path, "init", "--quiet", "--bare", "--initial-branch=main", str(repositories.remote))
    git(tmp_path, "init", "--quiet", "--initial-branch=main", str(repositories.author))
    git(repositories.author, "remote", "add", "origin", str(repositories.remote))
    repositories.root.mkdir()
    (repositories.root / PIN_FILE).write_text(_PLACEHOLDER_PIN)
    (repositories.root / ".gitignore").write_text("/.evidence/\n")
    git(repositories.root, "init", "--quiet", "--initial-branch=main")
    commit_evidence(
        repositories,
        files={
            "cases/query.sql": "select 1;\n",
            _STORED_RECEIPTS_FILE: _STORED_RECEIPT,
            "translation/inputs.jsonl": '{"input": 1}\n',
        },
    )
    return repositories


def commit_evidence(
    repositories: EvidenceRepositories,
    *,
    files: dict[str, str],
    deleted: tuple[str, ...] = (),
    branch: str = "main",
) -> str:
    """Write `files` and delete `deleted` in the evidence, commit, push to `branch`: the SHA."""
    for name, text in files.items():
        path: Path = repositories.author / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    for name in deleted:
        (repositories.author / name).unlink()
    git(repositories.author, "add", "-A")
    git(repositories.author, "commit", "--quiet", "--allow-empty", "-m", "evidence")
    git(repositories.author, "push", "--quiet", "--force", "origin", f"HEAD:{branch}")
    return git(repositories.author, "rev-parse", "HEAD")


def environment(repositories: EvidenceRepositories) -> dict[str, str]:
    """The variables that point the evidence script at the throwaway repositories."""
    return {
        "SQLQUACK_ROOT": str(repositories.root),
        "EVIDENCE_REPOSITORY": str(repositories.remote),
        "EVIDENCE_DIR": str(repositories.clone),
    }


def point_at(repositories: EvidenceRepositories, monkeypatch: pytest.MonkeyPatch) -> None:
    """Point this process's environment, which the pin check passes on, at the repositories."""
    for name, value in environment(repositories).items():
        monkeypatch.setenv(name, value)
    for name in _SCRIPT_SWITCHES:
        monkeypatch.delenv(name, raising=False)


def evidence_script(
    repositories: EvidenceRepositories, *arguments: str, variables: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    """Run scripts/evidence.sh against the throwaway repositories, with extra `variables`."""
    inherited: dict[str, str] = dict(os.environ)
    for name in _SCRIPT_SWITCHES:
        inherited.pop(name, None)
    return subprocess.run(
        ("bash", str(EVIDENCE_SCRIPT), *arguments),
        env={**inherited, **environment(repositories), **(variables or {})},
        check=False,
        capture_output=True,
        text=True,
    )


def pin(repositories: EvidenceRepositories, *revision: str) -> None:
    """Pin `revision` (default the evidence main) in the checkout's evidence.toml, or raise."""
    evidence_script(repositories, "pin", *revision).check_returncode()


def pin_value(repositories: EvidenceRepositories, key: str) -> str:
    """The value of `key` in the checkout's evidence.toml."""
    return str(tomllib.loads((repositories.root / PIN_FILE).read_text())[key])


def set_pin(repositories: EvidenceRepositories, *, commit: str, content_sha256: str) -> None:
    """Write `commit` and `content_sha256` into the checkout's evidence.toml by hand."""
    (repositories.root / PIN_FILE).write_text(
        f'{PIN_HEADER}commit = "{commit}"\ncontent_sha256 = "{content_sha256}"\n'
    )


def snapshot(repositories: EvidenceRepositories, commit: str) -> Path:
    """Where the evidence script puts the snapshot of `commit`."""
    return repositories.root / ".evidence" / "pinned" / commit


def make_writable(path: Path) -> None:
    """Give the owner write access to everything under `path`, so tests can tamper or delete."""
    subprocess.run(("chmod", "-R", "u+w", str(path)), check=True)


def take_remote_offline(repositories: EvidenceRepositories) -> None:
    """Move the remote away, so every clone or fetch from it fails as it does offline."""
    repositories.remote.rename(repositories.remote.with_name("offline.git"))


def pipeline_listing(directory: Path) -> str:
    """The `<sha256>  <path>` lines whose SHA-256 is the content hash of `directory`."""
    return subprocess.run(
        ("bash", "-c", _LISTING_PIPELINE),
        cwd=directory,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def pipeline_content_hash(directory: Path) -> str:
    """The content hash the defining shell pipeline prints for `directory`."""
    output: str = subprocess.run(
        ("bash", "-c", f"{_LISTING_PIPELINE} | sha256sum"),
        cwd=directory,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return output.split()[0]


def file_tree(root: Path, *, files: dict[str, bytes], links: dict[str, str]) -> Path:
    """Write `files` and the symbolic links `links` under `root`, beside an empty directory."""
    (root / "empty-directory").mkdir(parents=True)
    for name, content in files.items():
        (root / name).parent.mkdir(parents=True, exist_ok=True)
        (root / name).write_bytes(content)
    for name, target in links.items():
        (root / name).symlink_to(target)
    return root


def _read_only(path: Path) -> bool:
    return path.stat().st_mode & _READ_ONLY_BITS == 0


def _outcome(*, result: subprocess.CompletedProcess[str], facts: dict[str, bool]) -> ScriptOutcome:
    return ScriptOutcome(
        return_code=result.returncode, stdout=result.stdout, stderr=result.stderr, facts=facts
    )


def path_while_unpinned(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Asks for the snapshot's path while evidence.toml holds the placeholder."""
    result: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    return _outcome(result=result, facts={"nothing on stdout": result.stdout == ""})


def pin_main_then_older(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Pins main, then the commit before it, checking each pin's values."""
    older: str = git(repositories.author, "rev-parse", "HEAD")
    latest: str = commit_evidence(repositories, files={"syntax/receipts/later.jsonl": "{}\n"})
    pinned_main: subprocess.CompletedProcess[str] = evidence_script(repositories, "pin")
    main_values: tuple[str, str] = (
        pin_value(repositories, "commit"),
        pin_value(repositories, "content_sha256"),
    )
    pinned_older: subprocess.CompletedProcess[str] = evidence_script(repositories, "pin", older)
    content: str = pin_value(repositories, "content_sha256")
    stamp: Path = snapshot(repositories, older).with_suffix(".verified")
    return _outcome(
        result=pinned_older,
        facts={
            "pinning main succeeds": pinned_main.returncode == 0,
            "main's commit and hash": main_values
            == (latest, pipeline_content_hash(snapshot(repositories, latest))),
            "the older commit": pin_value(repositories, "commit") == older,
            "the older hash": content == pipeline_content_hash(snapshot(repositories, older)),
            "Python's hash agrees": content_sha256(snapshot(repositories, older)) == content,
            "comment and repository kept": (repositories.root / PIN_FILE)
            .read_text()
            .startswith(PIN_HEADER),
            "the stamp holds the hash": stamp.read_text() == f"{content}\n",
        },
    )


def path_offline_after_first_fetch(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Fetches the pin into a fresh clone, then asks again with the remote gone."""
    pin(repositories)
    commit: str = pin_value(repositories, "commit")
    make_writable(repositories.root / ".evidence")
    subprocess.run(
        ("rm", "-rf", str(repositories.root / ".evidence"), str(repositories.clone)), check=True
    )
    fetched: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    take_remote_offline(repositories)
    offline: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    receipts: Path = snapshot(repositories, commit) / _STORED_RECEIPTS_FILE
    return _outcome(
        result=offline,
        facts={
            "the first run succeeds": fetched.returncode == 0,
            "both print the snapshot": fetched.stdout
            == offline.stdout
            == f"{snapshot(repositories, commit)}\n",
            "the offline run is silent": offline.stderr == "",
            "files are read-only": _read_only(receipts),
            "folders are read-only": _read_only(receipts.parent),
        },
    )


def _refresh(
    repositories: EvidenceRepositories, *, arguments: tuple[str, ...], variables: dict[str, str]
) -> ScriptOutcome:
    """Tampers with the snapshot, refreshes online and then offline: the offline outcome."""
    pin(repositories)
    commit: str = pin_value(repositories, "commit")
    receipts: Path = snapshot(repositories, commit) / _STORED_RECEIPTS_FILE
    make_writable(receipts.parent)
    receipts.write_text("tampered\n")
    stamped: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    tampered: str = receipts.read_text()
    fetch_head: Path = repositories.clone / ".git" / "FETCH_HEAD"
    fetch_head.unlink()
    refreshed: subprocess.CompletedProcess[str] = evidence_script(
        repositories, *arguments, variables=variables
    )
    fetched: bool = fetch_head.exists()
    take_remote_offline(repositories)
    offline: subprocess.CompletedProcess[str] = evidence_script(
        repositories, *arguments, variables=variables
    )
    return _outcome(
        result=offline,
        facts={
            "a stamped snapshot is not re-hashed": (stamped.returncode, tampered)
            == (0, "tampered\n"),
            "refreshing succeeds": refreshed.returncode == 0,
            "refreshing fetches": fetched,
            "refreshing extracts again": receipts.read_text() == _STORED_RECEIPT,
            "the snapshot is read-only again": _read_only(receipts.parent),
            "refreshing prints the snapshot": refreshed.stdout
            == offline.stdout
            == f"{snapshot(repositories, commit)}\n",
        },
    )


def refresh_by_variable(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Refreshes with REFRESH=1 on path."""
    return _refresh(repositories, arguments=("path",), variables={"REFRESH": "1"})


def refresh_by_command(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Refreshes with the refresh command."""
    return _refresh(repositories, arguments=("refresh",), variables={})


def path_after_clone_changes(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Checks out a newer main in the clone, edits it, and refreshes the pinned snapshot."""
    pin(repositories)
    commit: str = pin_value(repositories, "commit")
    commit_evidence(repositories, files={"syntax/receipts/newer.jsonl": "{}\n"})
    git(repositories.clone, "fetch", "--quiet", "origin")
    git(repositories.clone, "checkout", "--quiet", "-B", "main", "origin/main")
    (repositories.clone / _STORED_RECEIPTS_FILE).write_text("local edit\n")
    plain: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    result: subprocess.CompletedProcess[str] = evidence_script(
        repositories, "path", variables={"REFRESH": "1"}
    )
    pinned: Path = snapshot(repositories, commit)
    return _outcome(
        result=result,
        facts={
            "both print the pinned snapshot": plain.stdout == result.stdout == f"{pinned}\n",
            "the pinned hash holds": content_sha256(pinned)
            == pin_value(repositories, "content_sha256"),
            "no newer file": not (pinned / "syntax/receipts/newer.jsonl").exists(),
            "no local edit": (pinned / _STORED_RECEIPTS_FILE).read_text() == _STORED_RECEIPT,
        },
    )


def path_offline_without_commit(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Pins a commit the clone never fetched, then asks for it with the remote gone."""
    pin(repositories)
    unfetched: str = commit_evidence(repositories, files={"syntax/receipts/later.jsonl": "{}\n"})
    set_pin(repositories, commit=unfetched, content_sha256="1" * 64)
    take_remote_offline(repositories)
    result: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    message: str = (
        f"evidence: commit {unfetched[:7]} (pinned in evidence.toml) is not in "
        f"{repositories.clone}, and fetching it failed. Connect to the network and run "
        "'make evidence-refresh', or set EVIDENCE_DIR to a clone that contains it."
    )
    return _outcome(
        result=result,
        facts={
            "nothing on stdout": result.stdout == "",
            "exactly the one message": result.stderr.splitlines() == [message],
        },
    )


def path_with_wrong_hash(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Pins main's commit with a content hash its files do not have."""
    pin(repositories)
    commit: str = pin_value(repositories, "commit")
    actual: str = pin_value(repositories, "content_sha256")
    pinned: Path = repositories.root / ".evidence" / "pinned"
    make_writable(pinned)
    subprocess.run(("rm", "-rf", str(pinned)), check=True)
    set_pin(repositories, commit=commit, content_sha256="f" * 64)
    result: subprocess.CompletedProcess[str] = evidence_script(repositories, "path")
    return _outcome(
        result=result,
        facts={
            "nothing on stdout": result.stdout == "",
            "one message": len(result.stderr.splitlines()) == 1,
            "names the expected hash": f"expects {'f' * 64}" in result.stderr,
            "names the actual hash": f"hash to {actual}" in result.stderr,
            "leaves no extraction behind": os.listdir(pinned) == [],
        },
    )


def path_of_working_tree(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Asks for the clone's unpinned working tree with EVIDENCE=working."""
    pin(repositories)
    result: subprocess.CompletedProcess[str] = evidence_script(
        repositories, "path", variables={"EVIDENCE": "working"}
    )
    return _outcome(
        result=result, facts={"prints the clone": result.stdout == f"{repositories.clone}\n"}
    )


def clone_then_clone_offline(repositories: EvidenceRepositories) -> ScriptOutcome:
    """Clones the evidence, then clones again with the remote gone."""
    first: subprocess.CompletedProcess[str] = evidence_script(repositories, "clone")
    take_remote_offline(repositories)
    second: subprocess.CompletedProcess[str] = evidence_script(repositories, "clone")
    return _outcome(
        result=second,
        facts={
            "the first clone succeeds": first.returncode == 0,
            "nothing is checked out": os.listdir(repositories.clone) == [".git"],
            "the clone has the evidence": git(repositories.clone, "rev-parse", "origin/main")
            == git(repositories.author, "rev-parse", "HEAD"),
        },
    )


def leave_placeholder(_: EvidenceRepositories) -> None:
    """Leaves evidence.toml's placeholder in place."""


def pin_main(repositories: EvidenceRepositories) -> None:
    """Pins the evidence main."""
    pin(repositories)


def malform_commit(repositories: EvidenceRepositories) -> None:
    """Writes a commit that is not a full SHA."""
    set_pin(repositories, commit="abc123", content_sha256=_PLACEHOLDER_HASH)


def tamper_with_snapshot(repositories: EvidenceRepositories) -> None:
    """Pins main, then rewrites a receipts file in its verified snapshot."""
    pin(repositories)
    receipts: Path = snapshot(repositories, pin_value(repositories, "commit"))
    make_writable(receipts)
    (receipts / _STORED_RECEIPTS_FILE).write_text('{"case_id": "query.tampered"}\n')


def pin_unfetched_offline(repositories: EvidenceRepositories) -> None:
    """Pins a commit the clone never fetched, and takes the remote away."""
    pin(repositories)
    unfetched: str = commit_evidence(repositories, files={"syntax/receipts/later.jsonl": "{}\n"})
    set_pin(repositories, commit=unfetched, content_sha256="1" * 64)
    take_remote_offline(repositories)


def append_receipt(repositories: EvidenceRepositories) -> str:
    """Appends a receipt to the pinned receipts file and adds a results file."""
    commit_evidence(
        repositories,
        files={
            _STORED_RECEIPTS_FILE: f'{_STORED_RECEIPT}{{"case_id": "query.later"}}\n',
            "translation/results/new.json": "{}\n",
        },
    )
    return "origin/main"


def edit_case(repositories: EvidenceRepositories) -> str:
    """Rewrites a case, which is not recorded evidence."""
    commit_evidence(repositories, files={"cases/query.sql": "select 2;\n"})
    return "origin/main"


def rewrite_receipt(repositories: EvidenceRepositories) -> str:
    """Rewrites the pinned receipts file."""
    commit_evidence(repositories, files={_STORED_RECEIPTS_FILE: '{"case_id": "query.other"}\n'})
    return "origin/main"


def delete_receipt(repositories: EvidenceRepositories) -> str:
    """Deletes the pinned receipts file."""
    commit_evidence(repositories, files={}, deleted=(_STORED_RECEIPTS_FILE,))
    return "origin/main"


def rewrite_inputs(repositories: EvidenceRepositories) -> str:
    """Rewrites an inputs.jsonl outside any recorded-evidence folder."""
    commit_evidence(repositories, files={"translation/inputs.jsonl": '{"input": 2}\n'})
    return "origin/main"


def replace_history(repositories: EvidenceRepositories) -> str:
    """Force-pushes a main that keeps every file but does not descend from the pinned commit."""
    git(repositories.author, "checkout", "--quiet", "--orphan", "replacement")
    commit_evidence(repositories, files={"syntax/receipts/later.jsonl": _STORED_RECEIPT})
    return "origin/main"


def pin_side_branch(repositories: EvidenceRepositories) -> str:
    """Appends on a branch other than main, and pins that."""
    git(repositories.author, "checkout", "--quiet", "-b", "side")
    commit_evidence(
        repositories, files={"syntax/receipts/side.jsonl": _STORED_RECEIPT}, branch="side"
    )
    return "origin/side"


def pin_check_after(
    *, repositories: EvidenceRepositories, change: Change, monkeypatch: pytest.MonkeyPatch
) -> int:
    """Pin main and commit the pin as the base, apply `change`, pin again and run the check."""
    pin(repositories)
    git(repositories.root, "add", "-A")
    git(repositories.root, "commit", "--quiet", "-m", "pin")
    pin(repositories, change(repositories))
    point_at(repositories, monkeypatch)
    return run_pin_check(["--base", "HEAD", "--repository", str(repositories.root)])
