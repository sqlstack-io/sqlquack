"""Crate manifests read from the repository, for the rules that check cargo features."""

from __future__ import annotations

from fensu import RuleContext

from scripts.fensu_policy._helpers.dialect_features import read_crate_features
from scripts.fensu_policy.models import CrateFeatures


def crate_features(*, ctx: RuleContext, manifest_path: str) -> CrateFeatures:
    """Read the features and optional dependencies of a workspace crate's Cargo.toml."""
    manifest: str = (ctx.repo_root / manifest_path).read_text(encoding="utf-8")
    return read_crate_features(manifest_text=manifest)
