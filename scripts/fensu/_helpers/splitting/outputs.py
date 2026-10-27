"""The files a split writes: destinations, re-pointed modules, declarations and the crate root."""

from __future__ import annotations

from pathlib import Path

from scripts.fensu._helpers.splitting.emit import declaration_files, destination_text
from scripts.fensu._helpers.splitting.layout import default_doc, lib_text
from scripts.fensu.classes.crate import Crate
from scripts.fensu.classes.splitter import Splitter
from scripts.fensu.constants import FILE_DOCS, LIB_FILE, MOD_FILE


def split_outputs(
    *, splitter: Splitter, crate: Crate, src: Path, docs: dict[str, str]
) -> dict[str, str]:
    """Every file the split writes: destinations, re-pointed modules, declarations and the root."""
    outputs: dict[str, str] = {}
    for file, destination in sorted(splitter.destinations.items()):
        doc: str = FILE_DOCS.get(file, default_doc(file=file, splitter=splitter))
        outputs[file] = destination_text(splitter=splitter, destination=destination, doc=doc)
    for module in crate.modules.values():
        if module.inline is None and module.file not in splitter.moved_files:
            relative: str = module.file.relative_to(src).as_posix()
            outputs[relative] = splitter.rewrite(module=module, tokens=module.tokens)[0]
    suffix: str = f"/{MOD_FILE}"
    kept: dict[str, str] = {
        relative.removesuffix(suffix): text
        for relative, text in outputs.items()
        if relative.endswith(suffix)
    }
    outputs.update(declaration_files(files=outputs, docs=docs, kept=kept))
    outputs[LIB_FILE] = lib_text(splitter=splitter, crate=crate, outputs=outputs)
    return outputs
