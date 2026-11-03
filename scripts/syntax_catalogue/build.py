"""Build coverage/syntax/sources.toml and coverage/syntax/catalogue/<family>.toml.

Usage: python -m scripts.syntax_catalogue.build --cache DIR [--out coverage/syntax]

DIR holds the private page cache written by `scripts.syntax_catalogue.fetch`: fetch-log.jsonl and
raw/<path>.gz. Form ids already in the catalogue are kept for the same (source, grammar); a
form whose grammar no longer appears keeps its id with `status = "absent_from_source"`, so no id
is reused.
"""

from __future__ import annotations

import argparse
import gzip
import json
import pathlib
import re
import tomllib
from collections import defaultdict

from scripts.syntax_catalogue import extract as X

FAMILIES = (
    "query",
    "dml",
    "ddl",
    "transaction_session",
    "security_admin",
    "utility",
    "scripting",
    "functions",
    "expressions",
    "classes",
)
SESSION_TITLES = re.compile(
    r"^(BEGIN|START TRANSACTION|COMMIT|ROLLBACK|USE\b.*|SET|UNSET|ALTER SESSION|SHOW TRANSACTIONS|"
    r"SHOW LOCKS|SHOW PARAMETERS|SHOW VARIABLES|DESCRIBE RESULT|ABORT TRANSACTION)$",
    re.I,
)
DML_TITLES = re.compile(
    r"^(INSERT.*|MERGE|UPDATE|DELETE|TRUNCATE TABLE|COPY INTO.*|COPY FILES|PUT|GET|LIST|REMOVE)$",
    re.I,
)
UTILITY_TITLES = re.compile(r"^(EXPLAIN|CALL.*|EXECUTE .*|CANCEL.*)$", re.I)
ADMIN_WORDS = re.compile(
    r"\b(GRANT\w*|REVOKE|USERS?|ROLES?|POLIC(Y|IES)|INTEGRATIONS?|ACCOUNTS?|ORGANIZATION\w*|"
    r"CONNECTIONS?|FAILOVER GROUPS?|REPLICATION GROUPS?|RESOURCE MONITORS?|SECRETS?|SHARES?|"
    r"PRIVILEGES?|NETWORK RULES?)\b",
    re.I,
)
QUERY_TITLES = re.compile(r"^(SELECT|WITH)$", re.I)
GENERAL_EXPRESSION = re.compile(
    r"^(operators|data-type|identifier|literals|bind-variables|session-variables|schema-variables|"
    r"collation|expressions-|sql-format-models|date-time-input|binary-input|ternary-logic|"
    r"name-resolution|reserved-keywords|conventions|virtual-columns)"
)


def family_of(path: str, title: str) -> str | None:
    parts = path.removesuffix(".md").split("/")
    if parts[0] != "sql-reference":
        return None
    if len(parts) >= 3 and parts[1] == "sql":
        t = re.sub(r"\s+", " ", title).strip()
        if QUERY_TITLES.match(t):
            return "query"
        if SESSION_TITLES.match(t):
            return "transaction_session"
        if DML_TITLES.match(t):
            return "dml"
        if UTILITY_TITLES.match(t):
            return "utility"
        if ADMIN_WORDS.search(t):
            return "security_admin"
        return "ddl"
    if len(parts) >= 3 and parts[1] in ("functions", "stored-procedures"):
        return "functions"
    if len(parts) >= 3 and parts[1] == "classes":
        return "classes"
    if len(parts) >= 3 and parts[1] == "snowflake-scripting":
        return "scripting"
    if len(parts) >= 3 and parts[1] == "constructs":
        return "query"
    if len(parts) == 2:
        name = parts[1]
        if name in ("operators-query", "constructs"):
            return "query"
        if name.startswith("functions-window-syntax"):
            return "functions"
        if name.startswith("constraints"):
            return "ddl"
        if GENERAL_EXPRESSION.match(name):
            return "expressions"
        return "expressions"
    return None


def source_id(path: str) -> str:
    return "doc." + path.removesuffix(".md").removeprefix("sql-reference/").replace("/", ".")


def page_slug(path: str) -> str:
    parts = path.removesuffix(".md").split("/")
    return ".".join(X.slug(p) for p in parts[2:]) if len(parts) > 2 else X.slug(parts[-1])


def leading_words(titles: list[str]) -> set[str]:
    words = {t.split()[0].upper() for t in titles if t}
    return words | {"DESC", "DESCRIBE", "LS", "RM", "WITH", "SELECT", "START", "BEGIN"}


def variant_names(grammars: list[str], headings: list[tuple[str, ...]]) -> list[str]:
    """Distinguishing words of each form after the words every form on the page shares."""
    words = [[w.upper() for w in X.bare_words(X.required_skeleton(g))] for g in grammars]
    common = 0
    while all(len(w) > common + 1 for w in words) and len({w[common] for w in words}) == 1:
        common += 1
    names = []
    for w, h in zip(words, headings, strict=True):
        rest = w[common:][:4]
        name = X.slug("-".join(rest))
        if not name:
            name = X.symbol_name(grammars[len(names)])
        if not name and len(h) > 1:
            name = X.slug(h[-1])
        names.append(name or "form")
    return names


def build(cache: pathlib.Path, out: pathlib.Path) -> dict:
    log = [json.loads(line) for line in (cache / "fetch-log.jsonl").read_text().splitlines()]
    latest = {}
    for rec in log:
        if rec["status"] == 200:
            latest[rec["path"]] = rec
    pages = {}
    for path, rec in sorted(latest.items()):
        md = gzip.decompress((cache / "raw" / (path + ".gz")).read_bytes()).decode()
        pages[path] = (rec, X.parse_page(md))
    leads = leading_words(
        [p.title for path, (_, p) in pages.items() if path.startswith("sql-reference/sql/")]
    )

    existing = load_existing(out)
    sources, forms_by_family, defs_by_family = [], defaultdict(list), defaultdict(list)
    used_ids = set(existing["ids"])
    for path, (rec, page) in pages.items():
        fam = family_of(path, page.title)
        lead = leads if path.startswith("sql-reference/sql/") else None
        prods = X.productions(page, lead)
        syntax_text = "\n\n".join(b.text for b in page.blocks)
        decisions = []
        stmts = [p for p in prods if p.kind == "statement"]
        # Identical grammar documented under several headings is one form.
        seen, unique = {}, []
        for p in stmts:
            if p.grammar in seen:
                seen[p.grammar].notes.append(f"also documented under '{' / '.join(p.heading[1:])}'")
                decisions.append("identical grammar under several headings merged into one form")
                continue
            seen[p.grammar] = p
            unique.append(p)
        definitions = {p.name: p.grammar for p in prods if p.kind == "definition"}
        groups = group_forms(fam, unique, decisions, set(definitions))
        for missing in page.syntax_sections_without_block:
            decisions.append(
                f"syntax heading '{missing}' has no syntax block; catalogued from prose"
            )
            groups.append(("prose", missing, [], (page.title, missing)))
        if any(p.elaborations for p in unique):
            decisions.append("elided block under the same heading merged into the earlier form")
        if fam is None:
            status = "out_of_scope"
        elif groups:
            status = "forms"
        else:
            status = "no_syntax_block"
        sid = source_id(path)
        sources.append(
            {
                "id": sid,
                "url": rec["url"].removesuffix(".md"),
                "title": page.title,
                "family": fam or "",
                "retrieved_utc": rec["retrieved_utc"],
                "page_sha256": rec["sha256"],
                "syntax_sha256": X.sha256(syntax_text),
                "syntax_blocks": len(page.blocks),
                "status": status,
                "decisions": sorted(set(decisions)),
            }
        )
        if fam is None or not groups:
            continue
        for name, grammar in definitions.items():
            defs_by_family[fam].append(
                {"id": f"{sid}::{name}", "source": sid, "name": name, "grammar": grammar}
            )
        forms_by_family[fam] += emit_forms(
            (f"sf.{fam}.{page_slug(path)}", fam, sid), groups, definitions, (existing, used_ids)
        )
    for fid, form in existing["forms"].items():
        if fid not in {f["id"] for fs in forms_by_family.values() for f in fs}:
            form = dict(form, status="absent_from_source")
            forms_by_family[form["family"]].append(form)
    return {"sources": sources, "forms": forms_by_family, "definitions": defs_by_family}


def emit_forms(page, groups, definitions, ids) -> list[dict]:
    """Form records for one page, keeping the id an earlier catalogue gave the same grammar."""
    (base, fam, sid), (existing, used_ids) = page, ids
    forms = []
    names = variant_names([g[2][0] if g[2] else "" for g in groups], [g[3] for g in groups])
    names = [
        X.slug(g[1]) if g[0] in ("statement", "call") and g[1] else n
        for g, n in zip(groups, names, strict=True)
    ]
    multi = len(groups) > 1
    for (kind, label, grammars, heading), name in zip(groups, names, strict=True):
        key = (sid, X.sha256(" || ".join(grammars)) if grammars else "prose:" + label)
        fid = existing["by_key"].get(key)
        if fid is None:
            fid = base if not multi else f"{base}.{name}"
            if fid in used_ids and len(heading) > 1:
                fid = f"{base}.{name}.{X.slug(heading[-1])}"
            n = 2
            while fid in used_ids:
                fid = f"{base}.{name}-{n}"
                n += 1
        used_ids.add(fid)
        forms.append(
            {
                "id": fid,
                "family": fam,
                "source": sid,
                "anchor": " / ".join(heading[1:]) or "Syntax",
                "kind": kind,
                "grammar": grammars,
                "grammar_sha256": key[1],
                "uses": sorted(
                    n
                    for n in definitions
                    if any(re.search(rf"\b{re.escape(n)}\b", g) for g in grammars)
                ),
                "status": "catalogued",
            }
        )
    return forms


def group_forms(fam, stmts, decisions, definitions):
    """(kind, label, grammars, heading) per form.

    Functions: plain calls under one heading are one form whose grammars are its signatures;
    each call with its own grammar (CAST ... AS, OVER, WITHIN GROUP, named arguments) is a form.
    Everything else: one form per statement production.
    """
    groups = []
    if fam in ("functions", "classes"):
        calls, headings = defaultdict(list), {}
        for p in stmts:
            if X.is_special_call(p.grammar):
                groups.append(("special", "", [p.grammar, *p.elaborations], p.heading))
            else:
                name = X.call_name(p.grammar)
                calls[name].append(p.grammar)
                headings.setdefault(name, p.heading)
        for name, grammars in reversed(list(calls.items())):
            if len(grammars) > 1:
                decisions.append("several call signatures of one function name form one call form")
            groups.insert(0, ("call", name, grammars, headings[name]))
        if any("=>" in g[2][0] for g in groups) and calls:
            decisions.append("function documents both positional and named-argument syntax")
        return groups
    for p in stmts:
        actions = action_alternatives(p.grammar, definitions)
        if actions:
            decisions.append(
                "statement whose tail alternates named action productions split per action"
            )
            for name, grammar in actions:
                groups.append(("statement", name, [grammar], p.heading))
        else:
            groups.append(("statement", "", [p.grammar, *p.elaborations], p.heading))
    return groups


def action_alternatives(grammar: str, definitions: set[str]) -> list[tuple[str, str]]:
    """`... { aAction | bAction }` where every alternative is a production the page defines."""
    m = re.search(r"\{\s*([A-Za-z_]\w*(?:\s*\|\s*[A-Za-z_]\w*)+)\s*\}\s*$", grammar)
    if not m:
        return []
    names = [a.strip() for a in m.group(1).split("|")]
    if not all(n in definitions for n in names):
        return []
    return [(n, grammar[: m.start()] + n) for n in names]


def load_existing(out: pathlib.Path) -> dict:
    forms, by_key = {}, {}
    for f in sorted((out / "catalogue").glob("*.toml")) if (out / "catalogue").exists() else []:
        for form in tomllib.loads(f.read_text()).get("form", []):
            forms[form["id"]] = form
            by_key[(form["source"], form["grammar_sha256"])] = form["id"]
    aliases = out / "aliases.toml"
    ids = set(forms)
    if aliases.exists():
        ids |= {a["from"] for a in tomllib.loads(aliases.read_text()).get("alias", [])}
    return {"forms": forms, "by_key": by_key, "ids": ids}


def toml_value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        if not v:
            return "[]"
        return "[\n" + "".join(f"  {toml_value(x)},\n" for x in v) + "]"
    return json.dumps(v, ensure_ascii=False)


def write_tables(path: pathlib.Path, header: str, table: str, rows: list[dict]) -> None:
    lines = [header.rstrip(), ""]
    for row in rows:
        lines.append(f"[[{table}]]")
        lines += [f"{k} = {toml_value(v)}" for k, v in row.items()]
        lines.append("")
    path.write_text("\n".join(lines))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", type=pathlib.Path, required=True)
    ap.add_argument("--out", type=pathlib.Path, default=pathlib.Path("coverage/syntax"))
    args = ap.parse_args()
    result = build(args.cache, args.out)
    (args.out / "catalogue").mkdir(parents=True, exist_ok=True)
    indexes = [
        {"name": f.name, "sha256": X.sha256(f.read_text())}
        for f in sorted((args.cache / "idx").glob("*"))
    ]
    write_tables(
        args.out / "indexes.toml",
        "# Documentation indexes the pinned page list was read from (llms.txt files).",
        "index",
        indexes,
    )
    write_tables(
        args.out / "sources.toml",
        "# Pinned Snowflake documentation pages; generated by scripts.syntax_catalogue.build.\n"
        "# Hashes are of the fetched Markdown and of its sqlsyntax blocks; the pages stay private.",
        "source",
        result["sources"],
    )
    for fam in FAMILIES:
        forms = sorted(result["forms"].get(fam, []), key=lambda f: f["id"])
        write_tables(
            args.out / "catalogue" / f"{fam}.toml",
            f"# Documented Snowflake syntax forms, family {fam}; generated by "
            "scripts.syntax_catalogue.build.",
            "form",
            forms,
        )
        defs = sorted(result["definitions"].get(fam, []), key=lambda d: d["id"])
        if defs:
            with open(args.out / "catalogue" / f"{fam}.toml", "a") as fh:
                for d in defs:
                    fh.write(
                        "[[definition]]\n"
                        + "".join(f"{k} = {toml_value(v)}\n" for k, v in d.items())
                        + "\n"
                    )
    counts = {fam: len(result["forms"].get(fam, [])) for fam in FAMILIES}
    print(json.dumps(counts))


if __name__ == "__main__":
    main()
