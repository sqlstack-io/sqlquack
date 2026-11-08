"""Pinned SQLite sources and the rules that turn them into catalogue forms."""

from __future__ import annotations

import re

SQLITE_VERSION: str = "3.54.1"
VERSION_NUMBER: str = "3540100"
SOURCE_ID: str = (
    "2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76"
)
DOWNLOAD_ROOT: str = "https://sqlite.org/2026/"
SITE_URL: str = "https://sqlite.org/"

DOCS: str = "doc"
SOURCE: str = "src"
AMALGAMATION: str = "amalgamation"
ARCHIVES: dict[str, tuple[str, str, str]] = {
    DOCS: (
        f"sqlite-doc-{VERSION_NUMBER}.zip",
        "e9f634ac28a4c7b48f5c389e4b08c039654ec1487512092b330ccfcfdc0816e0",
        "6ea5c0eff56c7fb305ca85e820070eacd96da44ae9edfb04d4c82d909540297b",
    ),
    SOURCE: (
        f"sqlite-src-{VERSION_NUMBER}.zip",
        "387e7f6cad4797b17848c07d453dc6606917bb356a6cfe5a3abafd38c240c71b",
        "877c6f4996ce7adb368686bce21638de9e3d91534832ad3d9d119037d1bc22ad",
    ),
    AMALGAMATION: (
        f"sqlite-amalgamation-{VERSION_NUMBER}.zip",
        "0a9c8e34352dad46006aa82836d5383bff9dc6bc9ce5a60553c05aa30e6ad1f9",
        "92281c86e2957f9b9305890f9ac04204caa7b677cc868dc326d942689df28e30",
    ),
}
DOCS_PREFIX: str = f"sqlite-doc-{VERSION_NUMBER}/"
SOURCE_PREFIX: str = f"sqlite-src-{VERSION_NUMBER}/"
AMALGAMATION_PREFIX: str = f"sqlite-amalgamation-{VERSION_NUMBER}/"
GRAMMAR_FILE: str = "src/parse.y"
KEYWORD_FILE: str = "tool/mkkeywordhash.c"
GRAMMAR_FILES: tuple[str, ...] = (GRAMMAR_FILE, KEYWORD_FILE)
AMALGAMATION_FILES: tuple[str, ...] = ("sqlite3.c", "sqlite3.h")

DOWNLOAD_DIR: str = "dl-{name}"
ENGINE_DIR: str = "engine"
LIBRARY_FILE: str = "libsqlite3.so"
ENGINE_EXTRACT: str = "engine.json"
FETCH_RECORD: str = "fetch.json"
UTC_FORMAT: str = "%Y-%m-%dT%H:%M:%SZ"
FETCH_TIMEOUT_SECONDS: int = 120
USER_AGENT: str = "sqlquack-syntax-catalogue/1"
COMPILER: str = "gcc"
COMPILE_FLAGS: tuple[str, ...] = (
    "-O2",
    "-fPIC",
    "-shared",
    "-DSQLITE_ENABLE_MATH_FUNCTIONS",
    "-DSQLITE_ENABLE_PERCENTILE",
)
LINK_FLAGS: tuple[str, ...] = ("-lm",)
ENGINE_QUERIES: dict[str, str] = {
    "functions": "SELECT name, builtin, type, narg FROM pragma_function_list ORDER BY 1, 3, 4",
    "pragmas": "SELECT name FROM pragma_pragma_list ORDER BY 1",
    "modules": "SELECT name FROM pragma_module_list ORDER BY 1",
    "collations": "SELECT name FROM pragma_collation_list ORDER BY 1",
    "compile_options": "SELECT compile_options FROM pragma_compile_options ORDER BY 1",
}

FORM_PREFIX: str = "sl"
FAMILIES: tuple[str, ...] = (
    "query",
    "dml",
    "ddl",
    "transaction_session",
    "security_admin",
    "utility",
    "functions",
    "expressions",
)
QUERY: str = "query"
DML: str = "dml"
DDL: str = "ddl"
SESSION: str = "transaction_session"
SECURITY: str = "security_admin"
UTILITY: str = "utility"
FUNCTIONS: str = "functions"
EXPRESSIONS: str = "expressions"

SYNTAX_DIR: str = "syntax/"
HTML_SUFFIX: str = ".html"
PAGE_GLOBS: tuple[str, ...] = (
    "syntax/*.html",
    "lang.html",
    "lang_*.html",
    "syntaxdiagrams.html",
    "pragma.html",
    "json1.html",
    "windowfunctions.html",
    "percentile.html",
    "datatype3.html",
    "stricttables.html",
    "partialindex.html",
    "vtab.html",
)
PAGE_EXCLUSIONS: tuple[tuple[str, str], ...] = (
    (
        "c3ref/*.html",
        "the C/C++ interface reference, one page per API object: host-language calls, not SQL",
    ),
    (
        "session/*.html",
        "the session extension's C interface reference: host-language calls, not SQL",
    ),
    (
        "releaselog/*.html",
        "release notes of every version; the syntax they announce is documented on the "
        "reference pages of the page set",
    ),
    (
        "{34to35,35to36,changes,chronology,news,oldnews,formatchng,pgszchng2016,"
        "pressrelease-20071212,cves,lts,versionnumbers,version3}.html",
        "release history, news, version numbering and compatibility notes, with no syntax of "
        "their own",
    ),
    (
        "{capi3,capi3ref,c_interface,cintro,backup,bindptr,unlock_notify,malloc,errlog,"
        "rescode,threadsafe,sharedcache,vfs,loadext,appfunc,tclsqlite,session,sessionintro,"
        "inmemorydb,uri,profile}.html",
        "the C, Tcl and extension-loading interfaces (prepared statements, result codes, VFS, "
        "URI and in-memory filenames, application-defined functions): host-language calls and "
        "filename strings, not SQL syntax; load_extension() is catalogued from lang_corefunc",
    ),
    (
        "{carray,series,csv,completion,zipfile,sqlar,spellfix1,unionvtab,swarmvtab,dbstat,"
        "dbpage,bytecodevtab,memstat,stmt,base64,base85,uintcseq,cksumvfs,diskusedfunc,rbu,"
        "recovery,fts3,fts5,rtree,geopoly,vec1,vec1doc,vec1ref,vec1bench,vtablist,tcljson,"
        "imposter}.html",
        "extensions, virtual tables, collations and functions that exist only in a build "
        "configured for them, in the CLI or as loadable extensions, not in the pinned library "
        "(built with SQLITE_ENABLE_MATH_FUNCTIONS and SQLITE_ENABLE_PERCENTILE only); their "
        "CREATE VIRTUAL TABLE spelling is the generic create-virtual-table-stmt and their "
        "table-valued calls the generic table-function syntax, both catalogued",
    ),
    (
        "{cli,climode,cliprompt,sqlite,sqldiff,sqlanalyze,dbhash,rsync}.html",
        "the command-line shell and utility programs: dot-commands and command-line options, "
        "not SQL syntax",
    ),
    (
        "{arch,atomiccommit,fileformat,fileformat2,opcode,vdbe,optoverview,queryplanner,"
        "queryplanner-ng,eqp,np1queryprob,testing,th3,assert,cpu,footprint,mmap,wal,walformat,"
        "lockingv3,isolation,tempfiles,psow,howtocorrupt,debugging,whybytecode,howitworks,"
        "onefile,transactional,shortnames,lemon,requirements,qmplan,hirely,speed,"
        "fasterthanfs,intern-v-extern-blob,useovernet,undoredo,staleexpridx}.html",
        "internals, file formats, the query planner, testing and performance: EXPLAIN QUERY "
        "PLAN output, bytecode and file layouts, not SQL syntax (EXPLAIN is catalogued from "
        "lang_explain and sql-stmt)",
    ),
    (
        "{howtocompile,compile,custombuild,amalgamation,amalg-v-canon,mingw,getthecode,"
        "download,privatebranch,whynotgit,whyc}.html",
        "building and obtaining SQLite: compile-time options and source trees, not SQL syntax; "
        "the options that change the grammar (SQLITE_ENABLE_UPDATE_DELETE_LIMIT, "
        "SQLITE_ENABLE_ORDERED_SET_AGGREGATES) are recorded in audit.toml and limits.toml",
    ),
    (
        "{about,index,docs,doclist,sitemap,syntax,keyword_index,doc_backlink_crossref,"
        "doc_keyword_crossref,doc_pagelink_crossref,doc_target_crossref,features,famous,"
        "mostdeployed,books,support,prosupport,consortium,consortium_agreement-20071201,crew,"
        "copyright,copyright-release,codeofconduct,codeofethics,hp1,security,whentouse,"
        "serverless,selfcontained,zeroconf,appfileformat,aff_short,affcase1,locrsf,"
        "quickstart,faq,glossary}.html",
        "project, index, licensing and overview pages that link to the reference; their "
        "examples restate catalogued statements",
    ),
    (
        "{different,quirks,nulls,flextypegood,floatingpoint,datatypes,rowidtable,"
        "withoutrowid,autoinc,gencol,rowvalue,foreignkeys,expridx,conflict,schematab,"
        "deterministic,printf,invalidutf,nulinstr,limits,omitted,fullsql}.html",
        "guides to features whose syntax is in the catalogued diagrams and pages (WITHOUT "
        "ROWID and STRICT in table-options, AUTOINCREMENT and GENERATED ALWAYS in "
        "column-constraint, row values in expr, foreign keys in foreign-key-clause, conflict "
        "resolution in conflict-clause, format() from lang_corefunc), SQLite version 2 "
        "datatypes, and prose on NULL handling, floating point, limits and unimplemented SQL",
    ),
)
SVG: re.Pattern[str] = re.compile(r"<svg\b.*?</svg>", re.S)
PIKCHR_CLASS: str = 'class="pikchr"'
SVG_TEXT: re.Pattern[str] = re.compile(r"<text\b([^>]*)>([^<]*)</text>")
ANNOTATION_STYLE: str = 'font-style="italic"'
TITLE: re.Pattern[str] = re.compile(r"<title>(.*?)</title>", re.S)
TAG: re.Pattern[str] = re.compile(r"<[^>]+>")
SPACES: re.Pattern[str] = re.compile(r"\s+")

DIAGRAM_FAMILIES: dict[str, str] = {
    **dict.fromkeys(
        (
            "compound-operator",
            "compound-select-stmt",
            "common-table-expression",
            "cte-table-name",
            "factored-select-stmt",
            "join-clause",
            "join-constraint",
            "join-operator",
            "ordering-term",
            "recursive-cte",
            "result-column",
            "select-core",
            "select-stmt",
            "simple-select-stmt",
            "table-or-subquery",
            "with-clause",
            "window-defn",
        ),
        QUERY,
    ),
    **dict.fromkeys(
        (
            "delete-stmt",
            "delete-stmt-limited",
            "insert-stmt",
            "qualified-table-name",
            "returning-clause",
            "update-stmt",
            "update-stmt-limited",
            "upsert-clause",
        ),
        DML,
    ),
    **dict.fromkeys(
        (
            "alter-table-stmt",
            "column-constraint",
            "column-def",
            "column-name-list",
            "conflict-clause",
            "create-index-stmt",
            "create-table-stmt",
            "create-trigger-stmt",
            "create-view-stmt",
            "create-virtual-table-stmt",
            "drop-index-stmt",
            "drop-table-stmt",
            "drop-trigger-stmt",
            "drop-view-stmt",
            "foreign-key-clause",
            "indexed-column",
            "table-constraint",
            "table-options",
        ),
        DDL,
    ),
    **dict.fromkeys(
        ("begin-stmt", "commit-stmt", "rollback-stmt", "savepoint-stmt", "release-stmt"),
        SESSION,
    ),
    **dict.fromkeys(
        (
            "analyze-stmt",
            "attach-stmt",
            "detach-stmt",
            "pragma-stmt",
            "pragma-value",
            "reindex-stmt",
            "vacuum-stmt",
            "sql-stmt",
            "sql-stmt-list",
        ),
        UTILITY,
    ),
    **dict.fromkeys(
        (
            "aggregate-function-invocation",
            "comment-syntax",
            "expr",
            "filter-clause",
            "frame-spec",
            "function-arguments",
            "literal-value",
            "numeric-literal",
            "over-clause",
            "raise-function",
            "signed-number",
            "simple-function-invocation",
            "type-name",
            "window-function-invocation",
        ),
        EXPRESSIONS,
    ),
}
PAGE_FAMILIES: dict[str, str] = {
    "lang.html": UTILITY,
    "lang_aggfunc.html": FUNCTIONS,
    "lang_altertable.html": DDL,
    "lang_analyze.html": UTILITY,
    "lang_attach.html": UTILITY,
    "lang_comment.html": EXPRESSIONS,
    "lang_conflict.html": DDL,
    "lang_corefunc.html": FUNCTIONS,
    "lang_createindex.html": DDL,
    "lang_createtable.html": DDL,
    "lang_createtrigger.html": DDL,
    "lang_createview.html": DDL,
    "lang_createvtab.html": DDL,
    "lang_datefunc.html": FUNCTIONS,
    "lang_delete.html": DML,
    "lang_detach.html": UTILITY,
    "lang_dropindex.html": DDL,
    "lang_droptable.html": DDL,
    "lang_droptrigger.html": DDL,
    "lang_dropview.html": DDL,
    "lang_explain.html": UTILITY,
    "lang_expr.html": EXPRESSIONS,
    "lang_indexedby.html": QUERY,
    "lang_insert.html": DML,
    "lang_keywords.html": EXPRESSIONS,
    "lang_mathfunc.html": FUNCTIONS,
    "lang_naming.html": EXPRESSIONS,
    "lang_reindex.html": UTILITY,
    "lang_replace.html": DML,
    "lang_returning.html": DML,
    "lang_savepoint.html": SESSION,
    "lang_select.html": QUERY,
    "lang_transaction.html": SESSION,
    "lang_update.html": DML,
    "lang_upsert.html": DML,
    "lang_vacuum.html": UTILITY,
    "lang_with.html": QUERY,
    "syntaxdiagrams.html": UTILITY,
    "pragma.html": UTILITY,
    "json1.html": FUNCTIONS,
    "windowfunctions.html": FUNCTIONS,
    "percentile.html": FUNCTIONS,
    "datatype3.html": EXPRESSIONS,
    "stricttables.html": DDL,
    "partialindex.html": DDL,
    "vtab.html": DDL,
}

SIGNATURE_BASES: dict[str, str] = {
    "lang_corefunc.html": "scalar",
    "lang_aggfunc.html": "aggregate",
    "lang_datefunc.html": "datetime",
    "lang_mathfunc.html": "math",
    "windowfunctions.html": "window",
    "json1.html": "json",
    "percentile.html": "percentile",
}
SIGNATURE_PAGES: tuple[str, ...] = tuple(SIGNATURE_BASES)
OPERATOR_LABELS: dict[str, str] = {"->": "arrow", "->>": "double-arrow"}
PRAGMA_PAGE: str = "pragma.html"
PRAGMA_BASE: str = "pragma"
ANCHOR_KEY: str = "anchor"
DT_SIGNATURES: re.Pattern[str] = re.compile(r"<dt><p><b>(.*?)</b>", re.S)
LI_SIGNATURES: re.Pattern[str] = re.compile(r"<li value='\d+'>\s*(.*?)\s*</li>", re.S)
BOLD_CALL_SIGNATURES: re.Pattern[str] = re.compile(
    r"<li>\s*<b>([a-z_]+\()</b>(.*?)<b>\)</b>\s*</li>", re.S
)
HEADING_SIGNATURES: re.Pattern[str] = re.compile(
    r"<h2 id=\"the_[a-z_]+_aggregate_function\"><span>[\d.]+ </span>The (.*?) aggregate function"
)
SIGNATURE_BREAK: re.Pattern[str] = re.compile(r"<br\s*/?>")
CALL: re.Pattern[str] = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)$", re.S)
ARGUMENT_NAME: re.Pattern[str] = re.compile(r"^[A-Za-z][A-Za-z0-9_-]*$")
ELLIPSIS: str = "..."
SIGNATURE_DEPTH: int = 0

OPERATOR_SIGNATURE: re.Pattern[str] = re.compile(r"^([a-z]+) (\S+) ([a-z]+)$")
CALL_STAR: str = "*"
EMPTY_PARENS: str = "( )"
PRAGMA_ANCHOR: re.Pattern[str] = re.compile(r'<a name="pragma_([a-z_0-9]+)"></a>')
PRAGMA_BLOCK: re.Pattern[str] = re.compile(r"<hr>\s*(?:<p>)?(.*?)(?:</p>|<p>)", re.S)
PRAGMA_KEYWORD: str = "PRAGMA"
PRAGMA_SCHEMA: str = "[ <schema-name> . ]"
PRAGMA_TOKEN: re.Pattern[str] = re.compile(r"'[^']*'|[A-Za-z_][A-Za-z0-9_]*|\d+|[=()|-]")
ITALIC: re.Pattern[str] = re.compile(r"<i>(.*?)</i>", re.S)
ITALIC_OPEN: str = "\x01"
ITALIC_CLOSE: str = "\x02"
ITALIC_SPLIT: re.Pattern[str] = re.compile("[\x01\x02]")
ITALIC_ARGUMENT: re.Pattern[str] = re.compile(r"\((\w+)\)")
SCHEMA_ITALICS: frozenset[str] = frozenset({"schema.", "schema"})
SCHEMA_SEPARATOR: str = "."
STATEMENT_END: str = ";"
NOTATION_BAR: str = "|"
WORD_START: re.Pattern[str] = re.compile(r"[A-Za-z_]")
PLACEHOLDER_CHOICE: re.Pattern[str] = re.compile(r"(<[^<>]+>) \| ([A-Z]+)")

DECISION_DIAGRAM: str = "the page's syntax diagram transcribed into the catalogue notation"
DECISION_RESTATED: str = (
    "restates diagrams the syntax/ pages own; listed in those forms' also_sources"
)
DECISION_SIGNATURES: str = "documented call signatures of one function name form one call form"
DECISION_SIGNATURE_OWNER: str = (
    "restates functions an earlier signature page catalogues; listed in their also_sources"
)
DECISION_PRAGMAS: str = "each PRAGMA's documented spellings form one statement form"
DECISION_PRAGMA_SCOPE: str = (
    "PRAGMAs the page documents only for other builds are out of scope (PRAGMA_OUT_OF_SCOPE)"
)
PRAGMA_DEBUG_ONLY: str = (
    "pragma.html says it is 'only available when the SQLITE_DEBUG compile-time option is used'; "
    "tool/mkpragmatab.tcl defines it under defined(SQLITE_DEBUG), and the pinned engine, built "
    "from the release amalgamation without SQLITE_DEBUG, does not list it in pragma_pragma_list"
)
PRAGMA_OUT_OF_SCOPE: dict[str, str] = {
    "parser_trace": PRAGMA_DEBUG_ONLY,
    "vdbe_addoptrace": PRAGMA_DEBUG_ONLY,
    "vdbe_debug": PRAGMA_DEBUG_ONLY,
    "vdbe_listing": PRAGMA_DEBUG_ONLY,
    "vdbe_trace": PRAGMA_DEBUG_ONLY,
    "stats": (
        "pragma.html says its intended use is 'only for testing and validation of SQLite' and that "
        "its behaviour is 'deliberately undocumented'; tool/mkpragmatab.tcl defines it under "
        "defined(SQLITE_DEBUG), and the pinned engine does not list it in pragma_pragma_list"
    ),
    "data_store_directory": (
        "pragma.html says it sets the variable 'windows operating-system interface backends use'; "
        "tool/mkpragmatab.tcl defines it under defined(_WIN32), and the pinned engine, a Linux "
        "build, does not list it in pragma_pragma_list"
    ),
    "legacy_file_format": (
        "pragma.html says it 'no longer functions' and 'has become a no-op'; tool/mkpragmatab.tcl "
        "no longer defines it, and the pinned engine does not list it in pragma_pragma_list"
    ),
}
DECISION_HAND: str = "catalogued by hand from {basis}"
DECISION_OVERRIDE: str = "signatures read by hand where the page's notation is not a plain call"
STATUS_FORMS: str = "forms"
STATUS_RESTATED: str = "restated"
STATUS_NO_SYNTAX: str = "no_syntax_block"
STATUS_OUT_OF_SCOPE: str = "out_of_scope"
PAGE_STATUSES: frozenset[str] = frozenset({STATUS_NO_SYNTAX, STATUS_OUT_OF_SCOPE})
HAND_FILE: str = "hand.toml"
HAND_BASES: frozenset[str] = frozenset({"table", "prose", "example", "heading", "list"})
HAND_KINDS: frozenset[str] = frozenset({"statement", "call", "special"})
KIND_STATEMENT: str = "statement"
KIND_CALL: str = "call"
KIND_SPECIAL: str = "special"
STATEMENT_FAMILIES: frozenset[str] = frozenset({QUERY, DML, DDL, SESSION, SECURITY, UTILITY})
DEFINITION_HEAD: re.Pattern[str] = re.compile(r"^<[^>]+>\s*::=\s*")
PLACEHOLDER: re.Pattern[str] = re.compile(r"<([^<>\s][^<>]*)>")
PLACEHOLDERS: re.Pattern[str] = re.compile(r"<[^<>]+>")
QUOTED_LITERAL: re.Pattern[str] = re.compile(r'^"(.+)"$')
NOTATION_MARKS: frozenset[str] = frozenset({"[", "]", "{", "}", "|", "..."})
DEFINITION_DEPTH: int = 8

ALIASES_FILE: str = "aliases.toml"
AUDIT_FILE: str = "audit.toml"
INDEXES_FILE: str = "indexes.toml"
SOURCES_FILE: str = "sources.toml"

NEWLINE: str = "\n"
ESCAPE: str = "\\"
BRACE_OPEN: str = "{"
BRACE_CLOSE: str = "}"
C_QUOTES: frozenset[str] = frozenset({'"', "'"})
BLOCK_COMMENT: re.Pattern[str] = re.compile(r"/\*.*?\*/", re.S)
LINE_COMMENT: re.Pattern[str] = re.compile(r"//[^\n]*")
DIRECTIVE_LINE: re.Pattern[str] = re.compile(r"^\s*%(ifdef|ifndef|if|else|endif)\b(.*)$")
DIRECTIVE_IFDEF: str = "ifdef"
DIRECTIVE_IFNDEF: str = "ifndef"
DIRECTIVE_IF: str = "if"
DIRECTIVE_ELSE: str = "else"
DIRECTIVE_ENDIF: str = "endif"
IF_TOKEN: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|&&|\|\||!|\(|\)")
IF_OR: str = "||"
IF_AND: str = "&&"
IF_NOT: str = "!"
IF_OPEN: str = "("
GRAMMAR_DEFINES: frozenset[str] = frozenset()
GRAMMAR_RULE: re.Pattern[str] = re.compile(
    r"\b([a-z][a-z0-9_]*)\s*(?:\([A-Za-z0-9_]*\))?\s*::=([^.]*)\.", re.S
)
GRAMMAR_SYMBOL: re.Pattern[str] = re.compile(
    r"([A-Za-z_][A-Za-z0-9_]*(?:\|[A-Za-z_][A-Za-z0-9_]*)*)(?:\([A-Za-z0-9_]*\))?"
)
TOKEN_CLASS: re.Pattern[str] = re.compile(r"%token_class\s+(\w+)\s+([\w|]+)\s*\.")
FALLBACK: re.Pattern[str] = re.compile(r"%fallback\s+([^.]*)\.")
WILDCARD: re.Pattern[str] = re.compile(r"%wildcard\s+(\w+)\s*\.")
START_RULE: str = "input"
STATEMENT_RULE: str = "cmd"
KEYWORD_TABLE: re.Pattern[str] = re.compile(r"aKeywordTable\[\]\s*=\s*\{(.*?)\n\};", re.S)
KEYWORD_ROW: re.Pattern[str] = re.compile(r'\{\s*"(\w+)",\s*"TK_(\w+)",')
FAMILY_FILES: str = "catalogue"
KEYWORD_WORD: re.Pattern[str] = re.compile(r"^[A-Z][A-Z0-9_]*$")
TOKEN_OR_WORD: re.Pattern[str] = re.compile(r"<[^>]*>|[\[\]{}]|[A-Za-z_][A-Za-z0-9_]*|\S")
OPTIONAL_OPEN: str = "["
OPTIONAL_CLOSE: str = "]"
TOKEN_ALTERNATIVE: str = "|"
ENGINE_NAME_WORD: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
MAX_SET: int = 60
MAX_EXPANSION: int = 4096
ANY_RUN: str = "…"
PUNCTUATION_TOKENS: dict[str, str] = {
    "LP": "(",
    "RP": ")",
    "COMMA": ",",
    "DOT": ".",
    "EQ": "=",
    "PLUS": "+",
    "MINUS": "-",
}
STANDARD_TYPES: re.Pattern[str] = re.compile(r"sqlite3StdType\[\]\s*=\s*\{(.*?)\};", re.S)
GENERATED_RULE_COUNT: re.Pattern[str] = re.compile(r"#define YYNRULE\s+(\d+)")
GROUP_CLOSERS: dict[str, str] = {"[": "]", "{": "}"}
GROUP_CLOSE_TOKENS: frozenset[str] = frozenset(GROUP_CLOSERS.values())
LINE_TOKEN: re.Pattern[str] = re.compile(
    r"<[^<>]*>|\.\.\.|\"[^\"]*\"|'[^']*'|[\[\]{}|]|[A-Za-z_][A-Za-z0-9_$]*|\S"
)
LINE_PUNCTUATION: frozenset[str] = frozenset(PUNCTUATION_TOKENS.values())
PLACEHOLDER_NAME: re.Pattern[str] = re.compile(r"^<([^<>\s][^<>]*)>$")
REPEAT: str = "..."
DIAGRAM_SOURCE: str = "doc.syntax."
INLINE_DEPTH: int = 4
INLINE_EXCLUDED: frozenset[str] = frozenset(
    {
        "expr",
        "select-stmt",
        "select-core",
        "factored-select-stmt",
        "simple-select-stmt",
        "compound-select-stmt",
        "sql-stmt",
        "sql-stmt-list",
    }
)
REVIEW_KEYWORD: str = "keyword"
REVIEW_NAME: str = "name"
REVIEW_RULE: str = "rule"
UNSEEN_RULES: tuple[str, ...] = (
    "ccons",
    "cmd",
    "exprlist",
    "key_opt",
    "nm",
    "seltablist",
    "tcons",
    "tconscomma",
    "trans_opt",
    "typetoken",
)
