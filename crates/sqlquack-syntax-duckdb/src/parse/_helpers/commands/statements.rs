//! `Statement` (common.gram): queries, DML and the common DDL as trees, every other statement a gap.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateFunction, FunctionBody, FunctionParameter,
};
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_type_func_name};
use crate::parse::constants::{MACRO_KIND, STATEMENT_STARTS};

/// The transformer's refusal of table and scalar definitions in one macro.
const MIXED_MACRO: &str = "Cannot mix table and scalar macro function definitions";

/// One `MacroDefinition` as a `CreateFunction` with its parameters, body and kind.
fn macro_overload(definition: (Vec<FunctionParameter>, Expression, bool)) -> CreateFunction {
    let (parameters, body, is_table) = definition;
    let mut node: CreateFunction = CreateFunction::new(String::new());
    node.parameters = parameters;
    node.body = Some(FunctionBody::Expression(body));
    node.has_parens = true;
    node.is_table_function = is_table;
    node
}

impl Parser<'_> {
    /// One statement.
    pub(crate) fn statement(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            if parser.is_char('(') {
                return parser.query();
            }
            let Some(word) = parser.word_at(0) else {
                return Err(parser.error());
            };
            match word.as_str() {
                "SELECT" | "FROM" | "VALUES" | "TABLE" | "PIVOT" | "PIVOT_WIDER" | "UNPIVOT"
                | "PIVOT_LONGER" | "DESCRIBE" | "DESC" | "SUMMARIZE" | "SHOW" => parser.query(),
                "WITH" => parser.with_statement_top(),
                "INSERT" => parser.insert_statement(None),
                "UPDATE" if parser.kw_at(1, "EXTENSIONS") && !parser.kw_at(2, "SET") => {
                    parser.update_extensions_statement()
                }
                "UPDATE" => parser.update_statement(None),
                "DELETE" => parser.delete_statement(None),
                "CREATE" => parser.create_statement(),
                "DROP" => parser.drop_statement(),
                "ALTER" => parser.alter_statement(),
                "MERGE" => parser.merge_statement(None),
                "ATTACH" => parser.attach_statement(),
                "COPY" => parser.copy_statement(),
                "PRAGMA" => parser.pragma_statement(),
                "USE" => parser.use_statement(),
                "RESET" => parser.reset_statement(),
                "TRUNCATE" => parser.truncate_statement(),
                "DETACH" => parser.detach_statement(),
                "INSTALL" => parser.install_statement(),
                "FORCE" => parser.force_statement(),
                "LOAD" => parser.load_statement(),
                "CHECKPOINT" => parser.checkpoint_statement(),
                "SET" => parser.set_statement(),
                "COMMENT" => parser.comment_statement(),
                "BEGIN" | "START" | "COMMIT" | "END" | "ROLLBACK" | "ABORT" => {
                    parser.transaction_statement()
                }
                "PREPARE" => parser.prepare_statement(),
                "EXECUTE" => parser.execute_statement(),
                "DEALLOCATE" => parser.deallocate_statement(),
                "CALL" => parser.call_statement(),
                "EXPLAIN" => parser.explain_statement(),
                "VACUUM" => parser.vacuum_statement(),
                "ANALYZE" | "ANALYSE" => parser.analyze_statement(),
                "EXPORT" => parser.export_statement(),
                "IMPORT" => parser.import_statement(),
                _ => parser.opaque_statement(),
            }
        })
    }

    /// A statement starting with `WITH`: a query, or a gap when the CTEs lead into DML.
    fn with_statement_top(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let with = self.ctes()?;
        match self.word_at(0).as_deref() {
            Some("INSERT") => return self.insert_statement(Some(with)),
            Some("UPDATE") => return self.update_statement(Some(with)),
            Some("DELETE") => return self.delete_statement(Some(with)),
            Some("MERGE") => return self.merge_statement(Some(with)),
            _ => {}
        }
        let query: Expression = self.query_rest(Some(with))?;
        Ok(self.respanned(first, query))
    }

    /// `CREATE [OR REPLACE] [TEMP] MACRO [IF NOT EXISTS] name (params) AS body, ...`.
    pub(crate) fn create_macro(&mut self) -> Parsing<Expression> {
        let first: usize = self.offset();
        self.at += 1;
        let or_replace: bool = self.eat_kws(&["OR", "REPLACE"]);
        let temporary: bool = self.eat_kw("TEMP") || self.eat_kw("TEMPORARY");
        if !temporary {
            self.eat_kw("PERSISTENT");
        }
        self.at += 1;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name = self.base_table_name()?;
        let start: usize = self.offset();
        let pivots: usize = self.pivot_entries;
        let mut node: CreateFunction = macro_overload(self.macro_definition()?);
        while self.eat_char(',') && self.is_char('(') {
            let overload: CreateFunction = macro_overload(self.macro_definition()?);
            if overload.is_table_function != node.is_table_function {
                self.refuse_later(MIXED_MACRO, start);
            }
            node.overloads
                .push(Expression::CreateFunction(Box::new(overload)));
        }
        self.pivot_entry_check(pivots, MACRO_KIND, first);
        node.name = name;
        node.or_replace = or_replace;
        node.if_not_exists = if_not_exists;
        node.temporary = temporary;
        Ok(Expression::CreateFunction(Box::new(node)))
    }

    /// `MacroDefinition`: `( params ) AS TABLE query | AS expression`.
    fn macro_definition(&mut self) -> Parsing<(Vec<FunctionParameter>, Expression, bool)> {
        self.expect_char('(')?;
        let mut parameters: Vec<FunctionParameter> = Vec::new();
        while !self.is_char(')') {
            parameters.push(self.macro_parameter()?);
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char(')')?;
        self.expect_kw("AS")?;
        if self.eat_kw("TABLE") {
            return Ok((parameters, self.query()?, true));
        }
        Ok((parameters, self.expression()?, false))
    }

    /// `MacroParameter`: `name [type] [:= default]`.
    fn macro_parameter(&mut self) -> Parsing<FunctionParameter> {
        let name: Identifier = self.name(is_type_func_name)?;
        let typed: bool =
            !(self.is_char(',') || self.is_char(')') || self.is_op(":=") || self.is_op("=>"));
        let data_type: DataType = match typed {
            true => self.data_type()?,
            false => DataType::Unknown,
        };
        let default: Option<Expression> = match self.eat_op(":=") || self.eat_op("=>") {
            true => Some(self.expression()?),
            false => None,
        };
        Ok(FunctionParameter {
            name: Some(name),
            data_type,
            mode: None,
            default,
            mode_text: None,
            span: SourceSpan::default(),
        })
    }

    /// A statement this slice does not build: a gap named by its first word, its text unchecked.
    pub(crate) fn opaque_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let word: String = self.word_at(0).unwrap_or_default().to_ascii_lowercase();
        let Some(construct) = STATEMENT_STARTS.iter().find(|start| **start == word) else {
            return Err(self.error());
        };
        self.at += 1;
        self.skip_statement_rest()?;
        self.gap(construct, start);
        Ok(Expression::Null(
            sqlquack_syntax::expressions::names::models::Null,
        ))
    }
}
