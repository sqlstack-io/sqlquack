//! Databricks' statements with no shared-tree node, checked against their documented syntax.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_word};
use crate::parse::constants::{CREATE_STREAMING_TABLE, SYNC};
use crate::scan::models::Token;

/// Named parts the patterns share; a lower-case word in a pattern names one of these or a slot.
pub(crate) const RULES: &[(&str, &str)] = &[
    ("owner", "[ SET ] OWNER TO id"),
    ("rename", "RENAME TO name"),
    ("tags", "SET TAGS props | UNSET TAGS keys"),
    (
        "name_tags",
        "SET TAGS ( { name = literal } ,... ) | UNSET TAGS ( name ,... )",
    ),
    ("if_exists", "IF EXISTS"),
    ("if_not_exists", "IF NOT EXISTS"),
    ("time_unit", "HOUR | HOURS | DAY | DAYS | WEEK | WEEKS"),
    (
        "predictive",
        "{ ENABLE | DISABLE | INHERIT } PREDICTIVE OPTIMIZATION",
    ),
    ("comment", "COMMENT str"),
    ("collation", "DEFAULT COLLATION id"),
    ("text", "str | id"),
    ("argument", "literal | name"),
    ("principals", "id ,..."),
    (
        "schedule",
        "SCHEDULE [ REFRESH ] { EVERY int time_unit | CRON str [ AT TIME ZONE str ] } \
         | TRIGGER ON UPDATE [ AT MOST EVERY expr ]",
    ),
    ("row_filter", "ROW FILTER name ON ( [ argument ,... ] )"),
    ("mask", "MASK name [ USING COLUMNS ( argument ,... ) ]"),
    (
        "column_action",
        "COMMENT str | SET MASK name [ USING COLUMNS ( argument ,... ) ] | DROP MASK | tags",
    ),
    (
        "pipeline_alter",
        "{ ADD | ALTER } schedule | DROP SCHEDULE | ALTER [ COLUMN ] name column_action \
         | SET ROW FILTER name ON ( [ argument ,... ] ) | DROP ROW FILTER | tags | owner",
    ),
    (
        "policy_target",
        "METASTORE | CATALOG name | SCHEMA name | TABLE name \
         | { MODEL SERVICE | MODEL PROVIDER SERVICE | MCP SERVICE | AGENT SERVICE | SKILL } name",
    ),
    (
        "signal_set",
        "SET { MESSAGE_ARGUMENTS | MESSAGE_TEXT } = expr",
    ),
    ("policy_on", "policy_target | name"),
    ("like", "[ LIKE ] str"),
    ("columns_for", "FOR { COLUMNS id ,... | ALL COLUMNS }"),
    ("policy_match", "MATCH COLUMNS { expr [ [ AS ] id ] } ,..."),
    (
        "policy_body",
        "ROW FILTER name TO principals [ EXCEPT principals ] FOR TABLES [ WHEN expr ] \
         [ policy_match ] [ USING COLUMNS ( expr ,... ) ] \
         | COLUMN MASK name TO principals [ EXCEPT principals ] FOR TABLES [ WHEN expr ] \
         [ policy_match ] ON COLUMN id [ USING COLUMNS ( expr ,... ) ] \
         | TO principals [ EXCEPT principals ] GRANT { id ... } ,... FOR id ... [ WHEN expr ]",
    ),
    (
        "tag_target",
        "CATALOG name | COLUMN id . name | EXTERNAL METADATA name | { FUNCTION | PROCEDURE } name \
         | { SCHEMA | DATABASE } name | SECRET name | TABLE name | VIEW name | VOLUME name",
    ),
    ("tag_value", "literal | id"),
    (
        "share_object",
        "MATERIALIZED VIEW name [ comment ] [ AS name ] | MODEL name [ comment ] [ AS name ] \
         | SCHEMA name [ comment ] | VIEW name [ comment ] [ AS name ] \
         | [ TABLE ] name [ comment ] [ share_partition ,... ] [ AS name ] \
         [ WITH HISTORY | WITHOUT HISTORY ]",
    ),
    (
        "share_partition",
        "PARTITION ( { id [ = literal | LIKE str ] } ,... )",
    ),
    ("cluster_spec", "( name ,... ) | AUTO | NONE"),
    (
        "expectation",
        "CONSTRAINT id EXPECT ( expr ) [ ON VIOLATION { FAIL UPDATE | DROP ROW } ]",
    ),
    (
        "pipeline_columns",
        "( { expectation | table_constraint | column } ,... )",
    ),
    (
        "pipeline_clause",
        "USING id | PARTITIONED BY ( { id [ type ] } ,... ) | CLUSTER BY cluster_spec | comment \
         | DEFAULT COLLATION id | TBLPROPERTIES props | REFRESH POLICY id [ STRICT ] | schedule \
         | WITH row_filter",
    ),
    ("cdc_columns", "* EXCEPT columns | name ,..."),
    ("cdc_source", "STREAM ( name ) | STREAM name | name"),
    (
        "auto_cdc",
        "FROM cdc_source KEYS columns [ IGNORE NULL UPDATES [ ON cdc_columns ] ] \
         [ APPLY AS DELETE WHEN expr ] [ APPLY AS TRUNCATE WHEN expr ] SEQUENCE BY expr \
         [ SYSTEM SEQUENCE BY expr ] [ COLUMNS cdc_columns ] \
         [ STORED AS { SCD TYPE int | BITEMPORAL } ] [ TRACK HISTORY ON cdc_columns ] \
         [ COLUMNS TO UPDATE expr ] \
         | [ ONCE ] FROM SNAPSHOT ( query ) [ WITH VERSION ( query ) ] KEYS columns \
         [ STORED AS SCD TYPE int ] [ TRACK HISTORY ON cdc_columns ]",
    ),
    (
        "flow",
        "FLOW { INSERT BY NAME query | AUTO CDC auto_cdc | REPLACE WHERE expr BY NAME query \
         | REPLACE USING columns SEQUENCE BY name BY NAME query }",
    ),
    (
        "procedure_parameter",
        "[ IN | OUT | INOUT ] id type [ DEFAULT expr ] [ comment ]",
    ),
    (
        "procedure_characteristic",
        "LANGUAGE SQL | SQL SECURITY { INVOKER | DEFINER } | NOT DETERMINISTIC | comment \
         | collation | MODIFIES SQL DATA",
    ),
];

/// A documented statement: its gap, the leading words selecting it, and its ordered patterns.
struct Form {
    construct: &'static str,
    leads: &'static [&'static [&'static str]],
    patterns: &'static [&'static str],
}

const FORMS: &[Form] = &[
    Form {
        construct: "alter_catalog",
        leads: &[&["ALTER", "CATALOG"]],
        patterns: &[
            "ALTER CATALOG [ name ] { collation | owner | name_tags | predictive \
             | OPTIONS props | SET MANAGED LOCATION str \
             | [ SET ] RETAIN DROPPED TO int time_unit }",
            "ALTER CATALOG name DROP CONNECTION { RESTRICT | FORCE }",
        ],
    },
    Form {
        construct: "alter_connection",
        leads: &[&["ALTER", "CONNECTION"]],
        patterns: &["ALTER CONNECTION name { owner | rename | OPTIONS props }"],
    },
    Form {
        construct: "alter_credential",
        leads: &[
            &["ALTER", "CREDENTIAL"],
            &["ALTER", "STORAGE", "CREDENTIAL"],
            &["ALTER", "SERVICE", "CREDENTIAL"],
        ],
        patterns: &["ALTER [ STORAGE | SERVICE ] CREDENTIAL name { rename | owner }"],
    },
    Form {
        construct: "alter_governed_tag",
        leads: &[&["ALTER", "GOVERNED"]],
        patterns: &["ALTER GOVERNED TAG id { SET DESCRIPTION text | SET VALUES ( id ,... ) }"],
    },
    Form {
        construct: "alter_external_location",
        leads: &[&["ALTER", "EXTERNAL"]],
        patterns: &[
            "ALTER EXTERNAL LOCATION name { rename | SET URL text [ FORCE ] \
             | SET STORAGE CREDENTIAL name | owner }",
        ],
    },
    Form {
        construct: "alter_materialized_view",
        leads: &[&["ALTER", "MATERIALIZED"]],
        patterns: &["ALTER MATERIALIZED VIEW name pipeline_alter"],
    },
    Form {
        construct: "alter_streaming_table",
        leads: &[&["ALTER", "STREAMING"]],
        patterns: &["ALTER STREAMING TABLE name pipeline_alter"],
    },
    Form {
        construct: "alter_provider",
        leads: &[&["ALTER", "PROVIDER"]],
        patterns: &["ALTER PROVIDER name { rename | owner }"],
    },
    Form {
        construct: "alter_recipient",
        leads: &[&["ALTER", "RECIPIENT"]],
        patterns: &[
            "ALTER RECIPIENT name { rename | owner | SET PROPERTIES props \
             | UNSET PROPERTIES keys }",
        ],
    },
    Form {
        construct: "alter_share",
        leads: &[&["ALTER", "SHARE"]],
        patterns: &["ALTER SHARE name { { ALTER | ADD } share_object \
             | REMOVE { MATERIALIZED VIEW | TABLE | SCHEMA | VIEW | MODEL } name | rename | owner }"],
    },
    Form {
        construct: "alter_volume",
        leads: &[&["ALTER", "VOLUME"]],
        patterns: &["ALTER VOLUME name { rename | owner | tags }"],
    },
    Form {
        construct: "create_catalog",
        leads: &[&["CREATE", "CATALOG"]],
        patterns: &[
            "CREATE CATALOG [ if_not_exists ] name [ { USING SHARE id . id \
             | MANAGED LOCATION str | RETAIN DROPPED FOR int time_unit | comment | collation \
             | OPTIONS props } ... ]",
        ],
    },
    Form {
        construct: "create_foreign_catalog",
        leads: &[&["CREATE", "FOREIGN", "CATALOG"]],
        patterns: &[
            "CREATE FOREIGN CATALOG [ if_not_exists ] name USING CONNECTION name \
             [ comment ] OPTIONS props",
        ],
    },
    Form {
        construct: "create_connection",
        leads: &[&["CREATE", "CONNECTION"]],
        patterns: &["CREATE CONNECTION [ if_not_exists ] name TYPE id OPTIONS props [ comment ]"],
    },
    Form {
        construct: "create_server",
        leads: &[&["CREATE", "SERVER"]],
        patterns: &["CREATE SERVER [ if_not_exists ] name TYPE id OPTIONS props [ comment ]"],
    },
    Form {
        construct: "create_governed_tag",
        leads: &[&["CREATE", "GOVERNED"]],
        patterns: &["CREATE GOVERNED TAG id [ DESCRIPTION text ] [ VALUES ( id ,... ) ]"],
    },
    Form {
        construct: "create_external_location",
        leads: &[&["CREATE", "EXTERNAL", "LOCATION"]],
        patterns: &["CREATE EXTERNAL LOCATION [ if_not_exists ] name URL text \
             WITH ( STORAGE CREDENTIAL name ) [ comment ]"],
    },
    Form {
        construct: "create_materialized_view",
        leads: &[
            &["CREATE", "MATERIALIZED"],
            &["CREATE", "OR", "REPLACE", "MATERIALIZED"],
        ],
        patterns: &[
            "CREATE MATERIALIZED VIEW [ if_not_exists ] name [ pipeline_columns ] \
             [ pipeline_clause ... ] [ AS ] query",
            "CREATE OR REPLACE MATERIALIZED VIEW name [ pipeline_columns ] \
             [ pipeline_clause ... ] [ AS ] query",
        ],
    },
    Form {
        construct: "create_streaming_table",
        leads: &[
            &["CREATE", "STREAMING"],
            &["CREATE", "OR", "REFRESH", "STREAMING"],
        ],
        patterns: &[
            "CREATE STREAMING TABLE [ if_not_exists ] name [ pipeline_columns ] \
             [ pipeline_clause ... ] [ flow | [ AS ] query ]",
            "CREATE OR REFRESH STREAMING TABLE name [ pipeline_columns ] \
             [ pipeline_clause ... ] [ flow | [ AS ] query ]",
        ],
    },
    Form {
        construct: "create_policy",
        leads: &[
            &["CREATE", "POLICY"],
            &["CREATE", "OR", "REPLACE", "POLICY"],
        ],
        patterns: &["CREATE [ OR REPLACE ] POLICY id ON policy_target [ comment ] policy_body"],
    },
    Form {
        construct: "create_procedure",
        leads: &[
            &["CREATE", "PROCEDURE"],
            &["CREATE", "OR", "REPLACE", "PROCEDURE"],
        ],
        patterns: &["CREATE [ OR REPLACE ] PROCEDURE [ if_not_exists ] name \
             ( [ procedure_parameter ,... ] ) [ procedure_characteristic ... ] AS statement"],
    },
    Form {
        construct: "create_recipient",
        leads: &[&["CREATE", "RECIPIENT"]],
        patterns: &[
            "CREATE RECIPIENT [ if_not_exists ] name [ USING ID id ] [ comment ] \
             [ PROPERTIES props ]",
        ],
    },
    Form {
        construct: "create_share",
        leads: &[&["CREATE", "SHARE"]],
        patterns: &["CREATE SHARE [ if_not_exists ] name [ comment ]"],
    },
    Form {
        construct: "create_volume",
        leads: &[&["CREATE", "VOLUME"], &["CREATE", "EXTERNAL", "VOLUME"]],
        patterns: &[
            "CREATE [ EXTERNAL ] VOLUME [ if_not_exists ] name [ LOCATION str ] \
             [ comment ]",
        ],
    },
    Form {
        construct: "drop_bloomfilter_index",
        leads: &[&["DROP", "BLOOMFILTER"]],
        patterns: &["DROP BLOOMFILTER INDEX ON [ TABLE ] name [ FOR COLUMNS columns ]"],
    },
    Form {
        construct: "drop_catalog",
        leads: &[&["DROP", "CATALOG"]],
        patterns: &["DROP CATALOG [ if_exists ] name [ RESTRICT | CASCADE ]"],
    },
    Form {
        construct: "drop_connection",
        leads: &[&["DROP", "CONNECTION"]],
        patterns: &["DROP CONNECTION [ if_exists ] name"],
    },
    Form {
        construct: "drop_credential",
        leads: &[
            &["DROP", "CREDENTIAL"],
            &["DROP", "STORAGE", "CREDENTIAL"],
            &["DROP", "SERVICE", "CREDENTIAL"],
        ],
        patterns: &["DROP [ STORAGE | SERVICE ] CREDENTIAL [ if_exists ] name [ FORCE ]"],
    },
    Form {
        construct: "drop_governed_tag",
        leads: &[&["DROP", "GOVERNED"]],
        patterns: &["DROP GOVERNED TAG id"],
    },
    Form {
        construct: "drop_external_location",
        leads: &[&["DROP", "EXTERNAL"]],
        patterns: &["DROP EXTERNAL LOCATION [ if_exists ] name [ FORCE ]"],
    },
    Form {
        construct: "drop_materialized_view",
        leads: &[&["DROP", "MATERIALIZED"]],
        patterns: &["DROP MATERIALIZED VIEW [ if_exists ] name"],
    },
    Form {
        construct: "drop_policy",
        leads: &[&["DROP", "POLICY"]],
        patterns: &["DROP POLICY [ if_exists ] id ON policy_target"],
    },
    Form {
        construct: "drop_procedure",
        leads: &[&["DROP", "PROCEDURE"]],
        patterns: &["DROP PROCEDURE [ if_exists ] name"],
    },
    Form {
        construct: "drop_provider",
        leads: &[&["DROP", "PROVIDER"]],
        patterns: &["DROP PROVIDER [ if_exists ] name"],
    },
    Form {
        construct: "drop_recipient",
        leads: &[&["DROP", "RECIPIENT"]],
        patterns: &["DROP RECIPIENT [ if_exists ] name"],
    },
    Form {
        construct: "drop_share",
        leads: &[&["DROP", "SHARE"]],
        patterns: &["DROP SHARE [ if_exists ] name"],
    },
    Form {
        construct: "drop_volume",
        leads: &[&["DROP", "VOLUME"]],
        patterns: &["DROP VOLUME [ if_exists ] name"],
    },
    Form {
        construct: "convert_to_delta",
        leads: &[&["CONVERT"]],
        patterns: &["CONVERT TO DELTA name [ NO STATISTICS ] \
             [ PARTITIONED BY ( { id type } ,... ) ]"],
    },
    Form {
        construct: "refresh_foreign",
        leads: &[&["REFRESH", "FOREIGN"]],
        patterns: &["REFRESH FOREIGN { CATALOG name | { SCHEMA | TABLE } name \
             [ RESOLVE DBFS LOCATION ] }"],
    },
    Form {
        construct: "refresh_materialized_view",
        leads: &[&["REFRESH", "MATERIALIZED"]],
        patterns: &["REFRESH MATERIALIZED VIEW name [ FULL ] [ SYNC | ASYNC ]"],
    },
    Form {
        construct: "refresh_table",
        leads: &[&["REFRESH", "STREAMING"], &["REFRESH", "TABLE"]],
        patterns: &["REFRESH [ STREAMING ] TABLE name [ FULL ] [ WHERE expr ] [ SYNC | ASYNC ]"],
    },
    Form {
        construct: "refresh_function",
        leads: &[&["REFRESH", "FUNCTION"]],
        patterns: &["REFRESH FUNCTION name"],
    },
    Form {
        construct: "set_tag",
        leads: &[&["SET", "TAG"]],
        patterns: &["SET TAG ON tag_target id [ = tag_value ]"],
    },
    Form {
        construct: "signal",
        leads: &[&["SIGNAL"], &["RESIGNAL"]],
        patterns: &[
            "SIGNAL { SQLSTATE [ VALUE ] str | name } [ signal_set ]",
            "RESIGNAL [ SQLSTATE [ VALUE ] str | name ] [ signal_set ]",
        ],
    },
    Form {
        construct: "open_cursor",
        leads: &[&["OPEN"]],
        patterns: &[
            "OPEN name [ USING { ( { expr [ [ AS ] id ] } ,... ) | { expr [ [ AS ] id ] } ,... } ]",
        ],
    },
    Form {
        construct: "fetch_cursor",
        leads: &[&["FETCH"]],
        patterns: &["FETCH [ [ NEXT ] FROM ] name INTO name ,..."],
    },
    Form {
        construct: "close_cursor",
        leads: &[&["CLOSE"]],
        patterns: &["CLOSE name"],
    },
    Form {
        construct: "analyze",
        leads: &[&["ANALYZE"]],
        patterns: &[
            "ANALYZE TABLE name [ partition ] COMPUTE { STATISTICS [ NOSCAN | columns_for ] \
             | DELTA STATISTICS }",
            "ANALYZE TABLE name COMPUTE STORAGE METRICS [ USING INVENTORY LOCATION str [ CONF str ] ]",
            "ANALYZE TABLE name DROP [ ALL | AUTO | MANUAL ] STATISTICS",
            "ANALYZE TABLES [ { FROM | IN } name ] COMPUTE STATISTICS [ NOSCAN ]",
        ],
    },
    Form {
        construct: "fsck",
        leads: &[&["FSCK"]],
        patterns: &["FSCK REPAIR TABLE name [ DRY RUN ] [ METADATA ONLY ]"],
    },
    Form {
        construct: "generate",
        leads: &[&["GENERATE"]],
        patterns: &["GENERATE id FOR TABLE name"],
    },
    Form {
        construct: "sync",
        leads: &[&["SYNC"]],
        patterns: &[
            "SYNC { SCHEMA | DATABASE | TABLE } name [ AS EXTERNAL ] FROM name [ SET OWNER id ] \
             [ DRY RUN ]",
        ],
    },
    Form {
        construct: "reorg",
        leads: &[&["REORG"]],
        patterns: &["REORG [ TABLE ] name [ WHERE expr ] APPLY ( { PURGE \
             | UPGRADE UNIFORM ( ICEBERG_COMPAT_VERSION = int ) } )"],
    },
    Form {
        construct: "call",
        leads: &[&["CALL"]],
        patterns: &["CALL name ( [ { id => expr | expr } ,... ] )"],
    },
    Form {
        construct: "clear_cache",
        leads: &[&["CLEAR"]],
        patterns: &["CLEAR CACHE"],
    },
    Form {
        construct: "list",
        leads: &[&["LIST"]],
        patterns: &["LIST str [ WITH ( CREDENTIAL name ) ] [ LIMIT int ]"],
    },
    Form {
        construct: "execute_immediate",
        leads: &[&["EXECUTE"]],
        patterns: &[
            "EXECUTE IMMEDIATE expr [ INTO name ,... ] [ USING { ( { expr [ [ AS ] id ] } ,... ) \
             | { expr [ [ AS ] id ] } ,... } ]",
        ],
    },
    Form {
        construct: "drop_variable",
        leads: &[
            &["DROP", "TEMPORARY", "VARIABLE"],
            &["DROP", "TEMPORARY", "VAR"],
            &["DROP", "VARIABLE"],
            &["DROP", "VAR"],
        ],
        patterns: &["DROP TEMPORARY { VARIABLE | VAR } [ IF EXISTS ] name"],
    },
    Form {
        construct: "describe_policy",
        leads: &[&["DESC", "POLICY"], &["DESCRIBE", "POLICY"]],
        patterns: &["{ DESC | DESCRIBE } POLICY name ON policy_on"],
    },
    Form {
        construct: "describe_credential",
        leads: &[
            &["DESC", "SERVICE", "CREDENTIAL"],
            &["DESCRIBE", "SERVICE", "CREDENTIAL"],
            &["DESC", "STORAGE", "CREDENTIAL"],
            &["DESCRIBE", "STORAGE", "CREDENTIAL"],
        ],
        patterns: &["{ DESC | DESCRIBE } { SERVICE | STORAGE } CREDENTIAL name"],
    },
    Form {
        construct: "describe_location",
        leads: &[
            &["DESC", "EXTERNAL", "LOCATION"],
            &["DESCRIBE", "EXTERNAL", "LOCATION"],
        ],
        patterns: &["{ DESC | DESCRIBE } EXTERNAL LOCATION name"],
    },
    Form {
        construct: "describe_governed_tag",
        leads: &[&["DESC", "GOVERNED"], &["DESCRIBE", "GOVERNED"]],
        patterns: &["{ DESC | DESCRIBE } GOVERNED TAG name [ AS JSON ]"],
    },
    Form {
        construct: "show_policies",
        leads: &[&["SHOW", "POLICIES"], &["SHOW", "EFFECTIVE"]],
        patterns: &["SHOW [ EFFECTIVE ] POLICIES ON policy_on"],
    },
    Form {
        construct: "show_shares",
        leads: &[&["SHOW", "SHARES"]],
        patterns: &["SHOW SHARES [ IN PROVIDER name ] [ like ]"],
    },
    Form {
        construct: "show_all_in_share",
        leads: &[&["SHOW", "ALL"]],
        patterns: &["SHOW ALL IN SHARE name"],
    },
    Form {
        construct: "show_groups",
        leads: &[&["SHOW", "GROUPS"]],
        patterns: &["SHOW GROUPS [ WITH { USER | GROUP } name ] [ like ]"],
    },
    Form {
        construct: "show_principals",
        leads: &[
            &["SHOW", "USERS"],
            &["SHOW", "PROVIDERS"],
            &["SHOW", "RECIPIENTS"],
        ],
        patterns: &["SHOW { USERS | PROVIDERS | RECIPIENTS } [ like ]"],
    },
    Form {
        construct: "show_volumes",
        leads: &[&["SHOW", "VOLUMES"]],
        patterns: &[
            "SHOW VOLUMES DROPPED [ { FROM | IN } name ] [ like ] [ LIMIT int ]",
            "SHOW VOLUMES [ { FROM | IN } name ] [ like ]",
        ],
    },
    Form {
        construct: "show_statistics",
        leads: &[&["SHOW", "STATISTICS"]],
        patterns: &["SHOW STATISTICS [ FROM | IN ] name [ columns_for ] AS JSON"],
    },
    Form {
        construct: "show_governed_tags",
        leads: &[&["SHOW", "GOVERNED"]],
        patterns: &["SHOW GOVERNED TAGS [ like ]"],
    },
    Form {
        construct: "show_securables",
        leads: &[
            &["SHOW", "CONNECTIONS"],
            &["SHOW", "CREDENTIALS"],
            &["SHOW", "EXTERNAL"],
            &["SHOW", "SERVICE"],
            &["SHOW", "STORAGE"],
        ],
        patterns: &[
            "SHOW { CONNECTIONS | CREDENTIALS | EXTERNAL LOCATIONS | SERVICE CREDENTIALS \
             | STORAGE CREDENTIALS } [ like ]",
        ],
    },
    Form {
        construct: "unset_tag",
        leads: &[&["UNSET"]],
        patterns: &["UNSET TAG ON tag_target id"],
    },
];

/// The rule `name` names, if any.
pub(crate) fn rule(name: &str) -> Option<&'static str> {
    RULES
        .iter()
        .find(|(rule, _)| *rule == name)
        .map(|(_, pattern)| *pattern)
}

/// The first token position in `tokens` where the words `names` follow, if any.
fn phrase_at(tokens: &[Token], names: &[&str]) -> Option<usize> {
    (0..tokens.len()).find(|at| phrase_starts(tokens, *at, names))
}

/// Whether the words `names` follow from token position `at` in `tokens`.
fn phrase_starts(tokens: &[Token], at: usize, names: &[&str]) -> bool {
    names.iter().enumerate().all(|(offset, name)| {
        tokens
            .get(at + offset)
            .is_some_and(|token| is_word(token, name))
    })
}

impl Parser<'_> {
    /// The documented statement starting here, checked and kept as its gap, if one begins here.
    pub(crate) fn documented_statement(&mut self) -> Option<Parsing<Expression>> {
        let form: &Form = FORMS.iter().find(|form| self.begins_form(form))?;
        let start: usize = self.offset();
        let first: usize = self.at;
        let statement: Expression = match self.checked_form(form.construct, start, form.patterns) {
            Ok(statement) => statement,
            Err(error) => return Some(Err(error)),
        };
        let check: Parsing<()> = match form.construct {
            CREATE_STREAMING_TABLE => self.auto_cdc_check(first),
            SYNC => self.sync_check(first),
            _ => Ok(()),
        };
        Some(check.map(|()| statement))
    }

    /// Whether one of `form`'s leading word sequences follows here.
    fn begins_form(&self, form: &Form) -> bool {
        form.leads.iter().any(|lead| self.words_follow(lead))
    }

    /// Whether the words `lead` follow here.
    fn words_follow(&self, lead: &[&str]) -> bool {
        lead.iter()
            .enumerate()
            .all(|(offset, word)| self.kw_at(offset, word))
    }

    /// Refuses `SYNC TABLE target FROM source` whose source is named other than its target.
    fn sync_check(&mut self, first: usize) -> Parsing<()> {
        let end: usize = self.at;
        self.at = first + 1;
        if !self.eat_kw("TABLE") {
            self.at = end;
            return Ok(());
        }
        let target: Vec<Identifier> = self.multipart_identifier()?;
        self.eat_kws(&["AS", "EXTERNAL"]);
        self.expect_kw("FROM")?;
        let source: Vec<Identifier> = self.multipart_identifier()?;
        self.at = end;
        let last = |parts: &[Identifier]| -> String {
            parts
                .last()
                .map(|part| part.name.to_ascii_lowercase())
                .unwrap_or_default()
        };
        match last(&target) == last(&source) {
            true => Ok(()),
            false => Err(self.error_with(
                &format!(
                    "[SYNC_SRC_TARGET_TBL_NOT_SAME] Source table name {} must be same as \
                     destination table name {}.",
                    last(&source),
                    last(&target)
                ),
                self.tokens[first].start,
                true,
            )),
        }
    }

    /// Refuses `SYSTEM SEQUENCE BY` without `STORED AS BITEMPORAL`, or the reverse.
    fn auto_cdc_check(&self, first: usize) -> Parsing<()> {
        let words: &[Token] = &self.tokens[first..self.at];
        let system: Option<usize> = phrase_at(words, &["SYSTEM", "SEQUENCE", "BY"]);
        let bitemporal: Option<usize> = phrase_at(words, &["STORED", "AS", "BITEMPORAL"]);
        let start: usize = self.tokens[first].start;
        match (system, bitemporal) {
            (Some(_), None) => Err(self.error_with(
                "[AUTO_CDC_SYSTEM_SEQUENCE_BY_REQUIRES_BITEMPORAL] SYSTEM SEQUENCE BY can only be \
                 used with STORED AS BITEMPORAL.",
                start,
                true,
            )),
            (None, Some(_)) => Err(self.error_with(
                "[AUTO_CDC_BITEMPORAL_REQUIRES_SYSTEM_SEQUENCE_BY] STORED AS BITEMPORAL requires a \
                 SYSTEM SEQUENCE BY clause.",
                start,
                true,
            )),
            _ => Ok(()),
        }
    }

    /// Spark's `REFRESH (stringLit | .*?)`: a resource path, quoted or as written.
    pub(crate) fn refresh_resource(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("REFRESH")?;
        if self.string_at(0) && self.ends_at(1) {
            self.string_lit()?;
        } else {
            let first: usize = self.at;
            self.skip_statement_rest()?;
            let path: &str = match self.at > first {
                true => &self.sql[self.tokens[first].start..self.tokens[self.at - 1].end],
                false => "",
            };
            if path.is_empty() {
                return Err(self.error_with(
                    "[INVALID_SQL_SYNTAX.EMPTY_RESOURCE_PATH] Resource paths cannot be empty in \
                     REFRESH statements.",
                    start,
                    true,
                ));
            }
            if path.contains([' ', '\n', '\r', '\t']) {
                return Err(self.error_with(
                    "[INVALID_SQL_SYNTAX.INVALID_REFRESH_RESOURCE_PATH] REFRESH statements cannot \
                     contain ' ', '\\n', '\\r', '\\t' inside unquoted resource paths.",
                    start,
                    true,
                ));
            }
        }
        self.gap("refresh", start);
        Ok(Expression::Null(
            sqlquack_syntax::expressions::names::models::Null,
        ))
    }
}
