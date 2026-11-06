"""Where the catalogue extract goes and the pg_catalog queries that produce it."""

from __future__ import annotations

from pathlib import Path

DATA_DIR: Path = Path("crates/sqlquack-semantics/src/lineage/_helpers/typing/postgres_catalog")
QUERIES: dict[str, str] = {
    "pg_proc": (
        "SELECT p.proname AS name, p.prokind AS kind, p.proretset AS returns_set, "
        "format_type(p.prorettype, NULL) AS return_type, "
        "COALESCE((SELECT string_agg(format_type(t, NULL), ',' ORDER BY o) "
        "FROM unnest(p.proargtypes::oid[]) WITH ORDINALITY AS a(t, o)), '') AS argument_types, "
        "CASE WHEN p.provariadic = 0 THEN '' ELSE format_type(p.provariadic, NULL) END AS variadic, "
        "p.pronargdefaults AS defaults, p.proisstrict AS strict, p.oid AS oid "
        "FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace "
        "WHERE n.nspname = 'pg_catalog' ORDER BY p.proname, p.oid"
    ),
    "pg_cast": (
        "SELECT format_type(c.castsource, NULL) AS source, format_type(c.casttarget, NULL) AS target, "
        "c.castcontext AS context, c.castmethod AS method FROM pg_cast c ORDER BY 1, 2"
    ),
    "pg_type": (
        "SELECT t.typname AS name, format_type(t.oid, NULL) AS formatted, t.typtype AS kind, "
        "t.typcategory AS category, t.typispreferred AS preferred, "
        "CASE WHEN t.typelem = 0 THEN '' ELSE format_type(t.typelem, NULL) END AS element "
        "FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace "
        "WHERE n.nspname = 'pg_catalog' ORDER BY t.typname"
    ),
    "pg_operator": (
        "SELECT o.oprname AS name, o.oprkind AS kind, "
        "CASE WHEN o.oprleft = 0 THEN '' ELSE format_type(o.oprleft, NULL) END AS left_type, "
        "CASE WHEN o.oprright = 0 THEN '' ELSE format_type(o.oprright, NULL) END AS right_type, "
        "format_type(o.oprresult, NULL) AS result_type FROM pg_operator o ORDER BY 1, 3, 4"
    ),
}
