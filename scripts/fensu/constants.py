"""Token kinds, Rust keywords and the placements the split applies."""

from __future__ import annotations

from scripts.fensu.models import Placement

WHITESPACE: str = "whitespace"
LINE_COMMENT: str = "line_comment"
BLOCK_COMMENT: str = "block_comment"
DOC_COMMENT: str = "doc_comment"
STRING: str = "string"
CHAR: str = "char"
LIFETIME: str = "lifetime"
IDENT: str = "ident"
NUMBER: str = "number"
PUNCT: str = "punct"

NON_DOC_COMMENT_KINDS: frozenset[str] = frozenset({LINE_COMMENT, BLOCK_COMMENT})

DOUBLE_QUOTE: str = '"'
SINGLE_QUOTE: str = "'"
BACKSLASH: str = "\\"
UNDERSCORE: str = "_"
DOT: str = "."
HASH: str = "#"
LINE_COMMENT_OPEN: str = "//"
BLOCK_COMMENT_OPEN: str = "/*"
BLOCK_COMMENT_CLOSE: str = "*/"
OUTER_LINE_DOC: str = "///"
INNER_LINE_DOC: str = "//!"
OUTER_BLOCK_DOC: str = "/**"
INNER_BLOCK_DOC: str = "/*!"
NOT_LINE_DOC: str = "////"
NOT_BLOCK_DOC: str = "/***"
EMPTY_BLOCK_COMMENT: str = "/**/"
RAW_STRING_PREFIXES: tuple[str, ...] = ("br", "cr", "r")
QUOTED_STRING_PREFIXES: tuple[str, ...] = ("b", "c")
BYTE_CHAR_PREFIX: str = "b'"
RAW_IDENTIFIER_PREFIX: str = "r#"

OPENING_BRACKETS: frozenset[str] = frozenset({"(", "[", "{"})
CLOSING_BRACKETS: frozenset[str] = frozenset({")", "]", "}"})

CRATE_NAME: str = "sqlquack_syntax"
CRATE_DOC: str = "SQLQuack's tokenizer, parser and syntax tree for Snowflake and DuckDB."
FILE_DOCS: dict[str, str] = {}
DIRECTORY_DOCS: dict[str, str] = {}
REWRITTEN_ROOTS: tuple[tuple[str, str, bool], ...] = (
    ("crates/sqlquack-syntax/tests", "sqlquack_syntax", True),
    ("crates/sqlquack-core/src", "sqlquack_syntax", True),
    ("crates/sqlquack-rules/src", "sqlquack_syntax", True),
    ("crates/sqlquack-blocks/src", "sqlquack_syntax", True),
    ("tools/sqlquack-polyglot-check/src", "sqlquack_syntax", True),
    ("crates/sqlquack/src", "sqlquack_syntax", True),
    ("fuzz/fuzz_targets", "sqlquack_syntax", True),
    ("crates/sqlquack-derive/src", "crate", False),
)
TRIVIA_KINDS: frozenset[str] = frozenset({WHITESPACE, LINE_COMMENT, BLOCK_COMMENT, DOC_COMMENT})
UNUSED_IMPORTS_LINT: str = "unused_imports"
TEST_PATH_PARTS: frozenset[str] = frozenset({"tests", "ported"})
USE_PREFIXES: tuple[str, ...] = ("use ", "pub use ", "pub(crate) use ")
ROLE_DIRECTORIES: frozenset[str] = frozenset({"_helpers", "main", "tests"})
ROLE_DOCS: dict[str, str] = {
    "models": "Data models of {area}.",
    "constants": "Constants of {area}.",
    "types": "Type aliases and traits of {area}.",
    "errors": "Errors of {area}.",
}

INNER_DOC: str = "inner_doc"
INNER_ATTRIBUTE: str = "inner_attr"
COMMENT: str = "comment"
MACRO: str = "macro"
INNER_DOC_PREFIXES: tuple[str, ...] = (INNER_LINE_DOC, INNER_BLOCK_DOC)

FN: str = "fn"
IMPL: str = "impl"
MOD: str = "mod"
USE: str = "use"
PUB: str = "pub"
CONST: str = "const"
STATIC: str = "static"
TRAIT: str = "trait"
FOR: str = "for"
WHERE: str = "where"
AS: str = "as"
MACRO_RULES: str = "macro_rules"
CRATE: str = "crate"
SUPER: str = "super"
SELF: str = "self"
GLOB: str = "*"

OPEN_PAREN: str = "("
CLOSE_PAREN: str = ")"
OPEN_BRACKET: str = "["
CLOSE_BRACKET: str = "]"
OPEN_BRACE: str = "{"
CLOSE_BRACE: str = "}"
OPEN_ANGLE: str = "<"
CLOSE_ANGLE: str = ">"
SEMICOLON: str = ";"
COLON: str = ":"
COMMA: str = ","
EQUALS: str = "="
MINUS: str = "-"
BANG: str = "!"
PATH_SEPARATOR: str = "::"

ITEM_MODIFIERS: frozenset[str] = frozenset({"const", "async", "unsafe", "extern", "default"})
FN_MODIFIERS: frozenset[str] = frozenset({"fn", "unsafe", "async", "extern"})
DECLARATION_KEYWORDS: frozenset[str] = frozenset(
    {"fn", "struct", "enum", "union", "trait", "mod", "type", "const", "static", "use"}
)
BRACED_ITEM_KEYWORDS: frozenset[str] = frozenset(
    {"fn", "struct", "enum", "union", "trait", "mod", "impl"}
)
NAMED_KINDS: frozenset[str] = frozenset(
    {"fn", "struct", "enum", "union", "trait", "type", "const", "static"}
)
SKIPPED_KINDS: frozenset[str] = frozenset({INNER_DOC, INNER_ATTRIBUTE, COMMENT, USE})
VALUE_KINDS: frozenset[str] = frozenset({CONST, STATIC})
PATH_ROOTS: frozenset[str] = frozenset({CRATE, SUPER, SELF})
INNER_GROUP_OPENERS: frozenset[str] = frozenset({OPEN_PAREN, OPEN_BRACKET})
INNER_GROUP_CLOSERS: frozenset[str] = frozenset({CLOSE_PAREN, CLOSE_BRACKET})
MACRO_CLOSERS: frozenset[str] = frozenset({CLOSE_PAREN, CLOSE_BRACKET})
IMPL_HEADER_ENDS: frozenset[str] = frozenset({FOR, WHERE})
STRUCT_BODY_STARTS: frozenset[str] = frozenset({OPEN_BRACE, OPEN_PAREN, SEMICOLON, WHERE})
BODYLESS_STRUCT_ENDS: frozenset[str] = frozenset({SEMICOLON, WHERE})
FIELD_GROUP_OPENERS: frozenset[str] = frozenset({OPEN_PAREN, OPEN_BRACKET, OPEN_BRACE, OPEN_ANGLE})
FIELD_GROUP_CLOSERS: frozenset[str] = frozenset({CLOSE_PAREN, CLOSE_BRACKET, CLOSE_BRACE})
VISIBILITY_TOKENS: frozenset[str] = frozenset({"pub", "(", ")", "crate", "super", "in", "self"})
SERDE_PATH_KEYS: frozenset[str] = frozenset(
    {"default", "skip_serializing_if", "serialize_with", "deserialize_with", "with"}
)

DROP: str = "-"
CRATE_VISIBILITY: str = "pub(crate)"
PUBLIC_MODULE: str = "pub mod"
PATH_ATTRIBUTE: str = "#[path"
ROOT_FILES: frozenset[str] = frozenset({"lib.rs", "mod.rs", "main.rs"})
LIB_FILE: str = "lib.rs"
MOD_STEM: str = "mod"
TESTS_STEM: str = "tests"
TESTS_FILE: str = "tests.rs"
TESTS_DIRECTORY_SUFFIX: str = "/tests"
MAIN_DIRECTORY: str = "main"
HELPERS_DIRECTORY: str = "_helpers"
PRIVATE_PREFIX: str = "_"
RUST_SUFFIX: str = ".rs"
DIRECTORY_SEPARATOR: str = "/"
REMOVE: str = "remove"
GATE: str = "gate"
COMPILER_MESSAGE: str = "compiler-message"
REASON: str = "reason"
MOD_FILE: str = "mod.rs"
STRUCT: str = "struct"
ENUM: str = "enum"
DEBUG_DERIVE: str = "Debug"
DERIVE_PREFIX: str = "#[derive("
MODELS_FILE: str = "models.rs"
SERDE_PATH_TOKENS: int = 2
PATH_SEPARATOR_TOKENS: int = 2
SELF_OR_SUPER: frozenset[str] = frozenset({SELF, SUPER})
DECLARATION_STEMS: frozenset[str] = frozenset({MOD_STEM, TESTS_STEM})

PARSER_HELPERS: str = "parser/_helpers"

PARSER_METHODS: tuple[tuple[str, str], ...] = (
    ("new", f"{PARSER_HELPERS}/statements/entry.rs"),
    ("parse_select_expressions", f"{PARSER_HELPERS}/queries/select.rs"),
    ("parse_table_expression_tail", f"{PARSER_HELPERS}/queries/tables.rs"),
    ("parse_group_by", f"{PARSER_HELPERS}/queries/clauses.rs"),
    ("parse_sample_clause", f"{PARSER_HELPERS}/queries/modifiers.rs"),
    ("parse_delete", f"{PARSER_HELPERS}/statements/dml.rs"),
    ("parse_create_table", f"{PARSER_HELPERS}/ddl/create_table.rs"),
    ("parse_snowflake_inline_table_option_value", f"{PARSER_HELPERS}/ddl/table_options.rs"),
    ("parse_constraint_definition", f"{PARSER_HELPERS}/ddl/constraints.rs"),
    ("parse_alter_action", f"{PARSER_HELPERS}/ddl/alter.rs"),
    ("collect_tsql_block_body_tokens", f"{PARSER_HELPERS}/ddl/scripting.rs"),
    ("parse_rm_command", f"{PARSER_HELPERS}/ddl/commands.rs"),
    ("parse_function_parameters", f"{PARSER_HELPERS}/expressions/operators.rs"),
    ("parse_comparison_tail", f"{PARSER_HELPERS}/expressions/comparisons.rs"),
    ("parse_colon_json_path_inner", f"{PARSER_HELPERS}/expressions/paths.rs"),
    ("parse_primary_slow", f"{PARSER_HELPERS}/expressions/primary.rs"),
    ("try_parse_registry_typed_function", f"{PARSER_HELPERS}/functions/registry.rs"),
    ("try_parse_phase3_typed_function", f"{PARSER_HELPERS}/functions/typed.rs"),
    ("parse_generic_function", f"{PARSER_HELPERS}/functions/generic.rs"),
    ("check_text_seq", f"{PARSER_HELPERS}/types/casts.rs"),
    ("parse_data_type_for_cast_inner", f"{PARSER_HELPERS}/types/data_types.rs"),
    ("check_from_keyword", f"{PARSER_HELPERS}/statements/analyze.rs"),
    ("parse_attach_detach", f"{PARSER_HELPERS}/statements/sessions.rs"),
    ("parse_group", f"{PARSER_HELPERS}/expressions/grouping.rs"),
    ("date_part_expr_name", f"{PARSER_HELPERS}/support/names.rs"),
    ("parse_table_parts", f"{PARSER_HELPERS}/support/tables.rs"),
)

PLACEMENTS: dict[str, Placement] = {
    "lib.rs": Placement(
        sections=(("enum:Dialect", "tokens/models.rs"),),
        overrides=(
            ("fn:parse_statements", "parser/main/parse_statements.rs"),
            ("fn:parse_statements_with", "parser/main/parse_statements_with.rs"),
            ("fn:tokenize", "lossless/main/tokenize.rs"),
        ),
    ),
    "error.rs": Placement(
        sections=(
            ("type:Result", "tokens/types.rs"),
            ("enum:Error", "tokens/errors.rs"),
            ("enum:ColumnResolutionTarget", "tokens/models.rs"),
        ),
        overrides=(
            ("struct:ValidationError", "tokens/errors.rs"),
            ("impl:ValidationError", "tokens/errors.rs"),
            ("mod:tests", "tokens/tests/test_error.rs"),
        ),
    ),
    "tokens.rs": Placement(
        sections=(
            ("fn:parse_dollar_string_token", "tokens/main/parse_dollar_string_token.rs"),
            ("struct:Span", "tokens/models.rs"),
            ("trait:TokenOutput", "tokens/types.rs"),
            ("impl:TokenOutput for Token", "tokens/models.rs"),
            ("static:DEFAULT_KEYWORDS", "tokens/_helpers/defaults.rs"),
            ("struct:TokenizerConfig", "tokens/models.rs"),
            ("trait:TokenizerCursor", "tokens/types.rs"),
            ("struct:AsciiCursor", "tokens/models.rs"),
        ),
        overrides=(("mod:tests", "tokens/tests/test_tokens.rs"),),
        methods=(
            (
                "impl:TokenizerState",
                (
                    ("new", "tokens/_helpers/scanning.rs"),
                    ("scan_string", "tokens/_helpers/literals.rs"),
                ),
            ),
        ),
    ),
    "guard.rs": Placement(
        sections=(
            ("const:DEFAULT_MAX_INPUT_BYTES", "guard/constants.rs"),
            ("struct:ParseOptions", "guard/models.rs"),
            ("fn:default_max_parser_depth", "guard/_helpers/defaults.rs"),
            ("struct:ComplexityGuardOptions", "guard/models.rs"),
            ("fn:parse_guard_error", "guard/_helpers/codes.rs"),
            ("fn:enforce_input", "guard/main/enforce_input.rs"),
            ("trait:GuardToken", "guard/types.rs"),
            ("impl:GuardToken for Token", "guard/_helpers/tokens.rs"),
            ("fn:enforce_token_slice", "guard/_helpers/tokens.rs"),
            ("fn:enforce_parser_tokens", "guard/main/enforce_parser_tokens.rs"),
            ("fn:enforce_parser_token_stats", "guard/main/enforce_parser_token_stats.rs"),
            ("fn:is_trivia_token", "guard/_helpers/tokens.rs"),
            ("fn:enforce_ast", "guard/main/enforce_ast.rs"),
            ("fn:enforce_ast_depth_only", "guard/_helpers/ast.rs"),
        ),
        overrides=(
            ("fn:is_guard_error", "guard/main/is_guard_error.rs"),
            ("fn:default_max_input_bytes", "guard/_helpers/defaults.rs"),
            ("fn:default_max_tokens", "guard/_helpers/defaults.rs"),
            ("fn:default_max_ast_nodes", "guard/_helpers/defaults.rs"),
            ("fn:default_max_ast_depth", "guard/_helpers/defaults.rs"),
            ("fn:default_max_parenthesis_depth", "guard/_helpers/defaults.rs"),
            ("fn:default_max_function_call_depth", "guard/_helpers/defaults.rs"),
            ("struct:TokenGuardStats", "guard/models.rs"),
            ("impl:TokenGuardStats", "guard/models.rs"),
            ("impl:Default for ComplexityGuardOptions", "guard/models.rs"),
            ("fn:enforce_tokens", "guard/_helpers/tokens.rs"),
            ("mod:token_guard_tests", "guard/tests/test_token_guard.rs"),
            ("struct:AstStats", "guard/models.rs"),
            ("enum:ConnectorKind", "guard/models.rs"),
        ),
    ),
    "lossless.rs": Placement(
        sections=(
            ("enum:TokenKind", "lossless/models.rs"),
            ("enum:TokenizeFailure", "lossless/errors.rs"),
            ("fn:tokenize_lossless", "lossless/main/tokenize_lossless.rs"),
            ("fn:tokenizer_config", "lossless/_helpers/trivia.rs"),
        ),
        overrides=(
            ("struct:CharOffsets", "lossless/models.rs"),
            ("impl:CharOffsets", "lossless/models.rs"),
        ),
    ),
    "modes.rs": Placement(
        sections=(
            ("enum:StrictFailure", "modes/errors.rs"),
            ("fn:parse_strict", "modes/main/parse_strict.rs"),
            ("struct:TolerantStatement", "modes/models.rs"),
            ("fn:parse_tolerant", "modes/main/parse_tolerant.rs"),
            ("fn:unparsed_whole", "modes/_helpers/statements.rs"),
        ),
    ),
    "dialects/mod.rs": Placement(
        sections=(
            ("enum:DialectType", "tokens/models.rs"),
            ("trait:DialectImpl", "parser/types.rs"),
            ("struct:CachedDialectConfig", "parser/models.rs"),
            ("fn:cached", f"{PARSER_HELPERS}/support/dialects.rs"),
            ("struct:Dialect", "parser/models.rs"),
        ),
        overrides=(("mod:tests", DROP),),
    ),
    "dialects/duckdb.rs": Placement(
        sections=(("struct:DuckDBDialect", "parser/models.rs"),),
        overrides=(("mod:tests", "parser/tests/test_duckdb.rs"),),
    ),
    "dialects/snowflake.rs": Placement(
        sections=(("struct:SnowflakeDialect", "parser/models.rs"),),
        overrides=(("mod:tests", DROP),),
    ),
    "dialects/generic.rs": Placement(sections=(("struct:GenericDialect", "parser/models.rs"),)),
    "ast_children.rs": Placement(
        sections=(
            ("enum:ChildPathSegment", "traversal/models.rs"),
            ("trait:AstNode", "traversal/types.rs"),
            ("fn:for_each_child", "traversal/main/for_each_child.rs"),
            ("fn:for_each_child_mut", "traversal/main/_for_each_child_mut.rs"),
            ("fn:for_each_child_untracked", "traversal/main/_for_each_child_untracked.rs"),
        ),
        overrides=(("mod:tests", "traversal/tests/test_ast_children.rs"),),
    ),
    "traversal.rs": Placement(
        sections=(
            ("type:NodeId", "traversal/types.rs"),
            ("struct:ParentInfo", "traversal/models.rs"),
            ("fn:child_location", "traversal/_helpers/context.rs"),
            ("struct:DfsIter", "traversal/models.rs"),
            ("trait:ExpressionWalk", "traversal/types.rs"),
            ("impl:ExpressionWalk for Expression", "traversal/_helpers/walk.rs"),
            ("fn:is_column", "traversal/_helpers/predicates.rs"),
        ),
        overrides=(("mod:tests", "traversal/tests/test_traversal.rs"),),
    ),
    "function_registry.rs": Placement(
        sections=(
            ("struct:FunctionSpec", "functions/names/models.rs"),
            ("const:FAITHFUL_KEYWORD_SYNTAX_NAMES", "functions/names/constants.rs"),
            (
                "fn:faithful_keyword_syntax_name",
                "functions/names/main/faithful_keyword_syntax_name.rs",
            ),
            ("const:NO_PAREN_FUNCTION_NAME_LIST", "functions/names/constants.rs"),
            ("static:FUNCTION_SPECS", "functions/names/_helpers/lookups.rs"),
            ("enum:TypedParseKind", "functions/typed/models.rs"),
            ("const:TYPED_FUNCTION_SPECS", "functions/typed/constants.rs"),
            ("static:TYPED_FUNCTION_SPEC_BY_CANONICAL", "functions/typed/_helpers/lookups.rs"),
            ("enum:TypedDispatchGroup", "functions/typed/models.rs"),
            ("const:TYPED_DISPATCH_GROUP_SPECS", "functions/typed/constants.rs"),
            ("static:TYPED_DISPATCH_GROUP_BY_NAME", "functions/typed/_helpers/lookups.rs"),
            ("enum:ParserDispatchBehavior", "functions/dispatch/models.rs"),
            ("const:PARSER_DISPATCH_SPECS", "functions/dispatch/constants.rs"),
            ("static:PARSER_DISPATCH_BEHAVIOR_BY_NAME", "functions/dispatch/_helpers/lookups.rs"),
            (
                "fn:parser_dispatch_behavior_by_name_upper",
                "functions/dispatch/main/parser_dispatch_behavior_by_name_upper.rs",
            ),
            (
                "fn:is_no_paren_function_name_upper",
                "functions/names/main/is_no_paren_function_name_upper.rs",
            ),
            ("fn:bare_user_is_function", "functions/names/main/bare_user_is_function.rs"),
            ("fn:is_aggregate_function_name_upper", "functions/names/_helpers/aggregates.rs"),
            (
                "fn:is_aggregate_function_name_for_dialect",
                "functions/names/main/is_aggregate_function_name_for_dialect.rs",
            ),
            ("fn:typed_function_spec_by_name_upper", "functions/typed/_helpers/lookups.rs"),
            (
                "fn:typed_function_spec_by_canonical_upper",
                "functions/typed/main/typed_function_spec_by_canonical_upper.rs",
            ),
            (
                "fn:canonical_typed_function_name_upper",
                "functions/typed/main/canonical_typed_function_name_upper.rs",
            ),
            (
                "fn:typed_dispatch_group_by_name_upper",
                "functions/typed/main/typed_dispatch_group_by_name_upper.rs",
            ),
        ),
        overrides=(("mod:tests", "functions/typed/tests/test_function_registry.rs"),),
    ),
    "expressions.rs": Placement(
        sections=(
            ("enum:Expression", "expressions/models.rs"),
            ("const:TYPED_EXPRESSION_FUNCTION", "expressions/constants.rs"),
            ("fn:type_assertion_result", "expressions/main/type_assertion_result.rs"),
            ("struct:Annotated", "expressions/models.rs"),
        ),
        overrides=(
            ("fn:default_true", "expressions/_helpers/serde_defaults.rs"),
            ("fn:is_true", "expressions/_helpers/serde_defaults.rs"),
            ("impl:Expression#1", "expressions/_helpers/accessors.rs"),
            ("impl:Expression#2", "expressions/_helpers/api.rs"),
            ("mod:tests", "expressions/tests/test_expressions.rs"),
        ),
    ),
    "parser.rs": Placement(
        sections=(
            ("type:MySqlTableOptions", "parser/types.rs"),
            ("struct:ViewModifiers", "parser/models.rs"),
            ("static:NO_PAREN_FUNCTIONS", f"{PARSER_HELPERS}/functions/token_sets.rs"),
            ("fn:convert_name_is_known_custom", f"{PARSER_HELPERS}/types/data_types.rs"),
            ("static:TYPE_TOKENS", f"{PARSER_HELPERS}/functions/token_sets.rs"),
            ("struct:Parser", "parser/models.rs"),
            ("impl:Parser#2", f"{PARSER_HELPERS}/support/tables.rs"),
            ("mod:tests", "parser/tests/test_parser.rs"),
            ("mod:join_marker_tests", "parser/tests/test_join_marker.rs"),
            ("mod:clickhouse_parser_regression_tests", DROP),
            ("mod:termination_tests", "parser/tests/test_termination.rs"),
            ("mod:explicit_eof_token_tests", "parser/tests/test_explicit_eof_token.rs"),
            ("fn:factorial", f"{PARSER_HELPERS}/expressions/comparisons.rs"),
        ),
        methods=(("impl:Parser#1", PARSER_METHODS),),
    ),
}

UNREACHABLE: str = "unreachable"
UNREACHABLE_MESSAGE: str = "the parser reached a state its checks rule out"
INTERNAL_ERROR: str = "crate::tokens::errors::Error::internal"
RESULT_RETURN: str = r"->\s*Result<"
ARGUMENT_SEPARATOR: str = ","
TAIL_CLOSERS: frozenset[str] = frozenset({"}", ","})
UNWRAP: str = "unwrap"
EXPECT: str = "expect"
UNWRAP_MESSAGE: str = '"a value the parser checked is missing"'
NAME_SEPARATOR: str = "_"
COMPARED_LITERAL_NAMES: dict[str, str] = {
    "d": "ODBC_DATE_ESCAPE",
    "t": "ODBC_TIME_ESCAPE",
    "ts": "ODBC_TIMESTAMP_ESCAPE",
    "s": "PYFORMAT_PARAMETER_SUFFIX",
    "e": "LOWERCASE_ESCAPE_STRING_PREFIX",
    "%": "PERCENT_SIGN",
    "@": "AT_SIGN",
}
CONSTANTS_FILE: str = "constants.rs"
CONSTANTS_MODULE: str = "constants"
DOMAIN_FILE_DEPTH: int = 2

MODEL_PLACEMENTS: dict[str, Placement] = {
    "expressions/tree/models.rs": Placement(
        sections=(
            ("enum:Expression", "expressions/tree/models.rs"),
            ("enum:Literal", "expressions/names/models.rs"),
            ("struct:Identifier", "expressions/names/models.rs"),
            ("struct:Select", "expressions/queries/models.rs"),
            ("struct:DirectoryInsert", "expressions/statements/models.rs"),
            ("struct:Alias", "expressions/operators/models.rs"),
            ("struct:Function", "expressions/functions/models.rs"),
            ("struct:From", "expressions/queries/models.rs"),
            ("struct:StructField", "expressions/types/models.rs"),
            ("struct:Command", "expressions/statements/models.rs"),
            ("struct:UnaryFunc", "expressions/typed_functions/models.rs"),
            ("struct:LambdaExpr", "expressions/operators/models.rs"),
            ("enum:OnCommit", "expressions/tables/models.rs"),
            ("struct:ColumnDef", "expressions/columns/models.rs"),
            ("struct:DropTable", "expressions/tables/models.rs"),
            ("struct:AlterTable", "expressions/columns/models.rs"),
            ("struct:CreateIndex", "expressions/tables/models.rs"),
            ("struct:ViewColumn", "expressions/schema_objects/models.rs"),
            ("struct:Truncate", "expressions/statements/models.rs"),
            ("struct:AlterView", "expressions/schema_objects/models.rs"),
            ("struct:Describe", "expressions/statements/models.rs"),
            ("struct:ByteString", "expressions/names/models.rs"),
            ("struct:AlterColumn", "expressions/columns/models.rs"),
            ("struct:Export", "expressions/operators/models.rs"),
            ("struct:Property", "expressions/tables/models.rs"),
            ("struct:Get", "expressions/statements/models.rs"),
            ("struct:DPipe", "expressions/functions/models.rs"),
            ("struct:JSON", "expressions/functions/models.rs"),
            ("struct:Map", "expressions/typed_functions/models.rs"),
            ("struct:XMLElement", "expressions/functions/models.rs"),
            ("struct:Zipf", "expressions/typed_functions/models.rs"),
            ("struct:Merge", "expressions/statements/models.rs"),
            ("enum:VerticaExpression", "expressions/statements/models.rs"),
        ),
    ),
}
MODEL_AREA_DOCS: dict[str, str] = {
    "expressions/names": "Names and literals: identifiers, columns, tables and literal values.",
    "expressions/queries": "Queries: SELECT, set operations, subqueries and their clauses.",
    "expressions/operators": "Operators, expression forms and the smaller clause nodes.",
    "expressions/functions": "Function calls and the built-in, JSON and XML function nodes.",
    "expressions/typed_functions": "Typed function nodes and the scalar function nodes.",
    "expressions/types": "Data types and the interval, array and struct nodes.",
    "expressions/tables": "Tables: CREATE and DROP TABLE, indexes and table properties.",
    "expressions/columns": "Columns: definitions, constraints and ALTER TABLE actions.",
    "expressions/schema_objects": "Schema objects: views, schemas, functions and sequences.",
    "expressions/statements": "Statements: data changes, commands, sessions and Vertica syntax.",
}
EXPRESSIONS_MOD: str = "expressions/mod.rs"
MOD_DECLARATION_PREFIXES: tuple[str, ...] = ("pub mod ", "pub(crate) mod ")
