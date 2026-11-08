//! `PivotStatement` and `UnpivotStatement` (pivot.gram): the simplified pivot forms of a query.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::queries::models::Pivot;
use sqlquack_syntax::expressions::statements::models::UnpivotColumns;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{column_from, in_list, joined_table, paren, tuple};

/// The transformer's refusal of a `PIVOT` with none of its clauses.
const PIVOT_WITHOUT_CLAUSES: &str = "PIVOT requires an ON, USING or GROUP BY clause";

impl Parser<'_> {
    /// `PivotStatement`: `PIVOT|PIVOT_WIDER TableRef [ON entries] [USING targets] [GROUP BY]`.
    pub(crate) fn pivot_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.at += 1;
        let this: Expression = self.pivot_source()?;
        let expressions: Vec<Expression> = match self.eat_kw("ON") {
            true => self.comma_list(|parser| parser.pivot_column_entry())?,
            false => Vec::new(),
        };
        let using: Vec<Expression> = match self.eat_kw("USING") {
            true => self.comma_list(|parser| parser.named_expression())?,
            false => Vec::new(),
        };
        let group: Option<Box<Expression>> = self.pivot_group_by()?;
        if expressions.is_empty() && using.is_empty() && group.is_none() {
            self.refuse_later(PIVOT_WITHOUT_CLAUSES, start);
        }
        Ok(Expression::Pivot(Box::new(Pivot {
            this,
            expressions,
            fields: Vec::new(),
            using,
            group,
            unpivot: false,
            into: None,
            alias: None,
            alias_columns: Vec::new(),
            include_nulls: None,
            default_on_null: None,
            with: None,
        })))
    }

    /// `UnpivotStatement`: `UNPIVOT|PIVOT_LONGER TableRef ON targets [IntoNameValues]`.
    pub(crate) fn unpivot_statement(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let this: Expression = self.pivot_source()?;
        self.expect_kw("ON")?;
        let expressions: Vec<Expression> = self.comma_list(|parser| parser.named_expression())?;
        let into: Option<Box<Expression>> = match self.kw("INTO") {
            true => Some(Box::new(self.name_values_into()?)),
            false => None,
        };
        Ok(Expression::Pivot(Box::new(Pivot {
            this,
            expressions,
            fields: Vec::new(),
            using: Vec::new(),
            group: None,
            unpivot: true,
            into,
            alias: None,
            alias_columns: Vec::new(),
            include_nulls: None,
            default_on_null: None,
            with: None,
        })))
    }

    /// `PivotGroupByList?`: `GROUP BY names` or `GROUP BY (names)`, as one tuple of columns.
    pub(crate) fn pivot_group_by(&mut self) -> Parsing<Option<Box<Expression>>> {
        if !(self.kw("GROUP") && self.kw_at(1, "BY")) {
            return Ok(None);
        }
        self.at += 2;
        let names: Vec<Identifier> = self.optional_parens_names()?;
        let columns: Vec<Expression> = names
            .into_iter()
            .map(|name| column_from(vec![name]))
            .collect();
        Ok(Some(Box::new(tuple(columns))))
    }

    /// The statement's `TableRef`, its joins folded into one source.
    fn pivot_source(&mut self) -> Parsing<Expression> {
        let (item, joins) = self.table_ref()?;
        Ok(match joins.is_empty() {
            true => item,
            false => joined_table(item, joins, None),
        })
    }

    /// `PivotColumnEntry`: `header IN (query|values|enum)`, kept once it parses, or else an expression.
    fn pivot_column_entry(&mut self) -> Parsing<Expression> {
        let listed = self.attempt(|parser| {
            let header: Expression = parser.primary_expression()?;
            parser.expect_kw("IN")?;
            if parser.is_char('(') && parser.starts_query_at(1) {
                parser.at += 1;
                let query: Expression = parser.query()?;
                parser.expect_char(')')?;
                parser.pivot_entries += 1;
                return Ok(in_list(header, Vec::new(), Some(query), false));
            }
            parser.pivot_values(header)
        });
        match listed {
            Ok(entry) => Ok(entry),
            Err(_) => {
                let entry: Expression = match self.expression()? {
                    boolean @ Expression::In(_) => paren(boolean),
                    other => other,
                };
                self.pivot_entries += 1;
                Ok(entry)
            }
        }
    }

    /// `PivotEntryCheck`: refuses a PIVOT read since entry `since` that takes its values from the data.
    pub(crate) fn pivot_entry_check(&mut self, since: usize, kind: &str, start: usize) {
        if self.pivot_entries > since {
            let message: String = format!(
                "PIVOT statements with pivot elements extracted from the data cannot be used in {kind}s."
            );
            self.refuse_later(&message, start);
        }
    }

    /// `PivotValueTarget` after `header IN`: a value list, or an enum's name marked as a field.
    pub(crate) fn pivot_values(&mut self, header: Expression) -> Parsing<Expression> {
        if self.is_char('(') {
            let values: Vec<Expression> = self.parenthesized(|parser| parser.named_expression())?;
            return Ok(in_list(header, values, None, false));
        }
        let mut entry: Expression = in_list(
            header,
            vec![column_from(vec![self.name(is_identifier)?])],
            None,
            false,
        );
        if let Expression::In(node) = &mut entry {
            node.is_field = true;
        }
        Ok(entry)
    }

    /// `IntoNameValues`: `INTO NAME name VALUE|VALUES names` or `(names)`.
    fn name_values_into(&mut self) -> Parsing<Expression> {
        self.expect_kw("INTO")?;
        self.expect_kw("NAME")?;
        let name: Identifier = self.col_id_or_string()?;
        if !(self.eat_kw("VALUE") || self.eat_kw("VALUES")) {
            return Err(self.error());
        }
        let values: Vec<Expression> = self
            .optional_parens_names()?
            .into_iter()
            .map(|value| column_from(vec![value]))
            .collect();
        Ok(Expression::UnpivotColumns(Box::new(UnpivotColumns {
            this: Box::new(column_from(vec![name])),
            expressions: values,
        })))
    }

    /// `OptionalParensNameList`: `(names)` or `names`, each a `ColIdOrString`.
    fn optional_parens_names(&mut self) -> Parsing<Vec<Identifier>> {
        match self.is_char('(') {
            true => self.parenthesized(|parser| parser.col_id_or_string()),
            false => self.comma_list(|parser| parser.col_id_or_string()),
        }
    }
}
