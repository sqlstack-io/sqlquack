# SQLQuack

SQLQuack is an early-stage SQLGlot-inspired Rust engine for warehouse SQL analysis and fail-closed DuckDB transpilation.

SQLQuack is not a broad `N x N` dialect transpiler. It is intended for callers that need structured facts about expanded warehouse SQL: diagnostics, output columns, references, lineage, and conservative type/nullability information.

DuckDB output is fail-closed. SQLQuack should only emit DuckDB SQL when the source semantics are expected to be preserved; unsupported or ambiguous constructs should produce diagnostics instead of best-effort rewrites.

Status: early development. APIs are unstable.
