//! `functionCall` (SqlBaseParser.g4), with `windowSpec`, `windowFrame` and `sortItem`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::WithinGroup;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::{
    NamedArgSeparator, NamedArgument, TableArgument,
};
use sqlquack_syntax::expressions::queries::models::{
    Ordered, Over, WindowFrame, WindowFrameBound, WindowFrameKind,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    aggregate, function, joined, ordered, over, subquery, table, window_function,
};

impl Parser<'_> {
    /// A call of `name`: `( [DISTINCT|ALL] args )` and its trailing clauses.
    pub(crate) fn function_call(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        self.expect_char('(')?;
        let mut distinct: bool = false;
        let mut args: Vec<Expression> = Vec::new();
        if !self.is_char(')') {
            if self.kw("DISTINCT") && !self.char_at(1, ')') && !self.char_at(1, ',') {
                self.at += 1;
                distinct = true;
            } else if self.kw("ALL") && !self.char_at(1, ')') && !self.char_at(1, ',') {
                self.at += 1;
            }
            args = self.comma_list(|parser| parser.function_argument())?;
        }
        self.expect_char(')')?;
        let mut within: Option<Vec<Ordered>> = None;
        if self.kw("WITHIN") && self.kw_at(1, "GROUP") && self.char_at(2, '(') {
            self.at += 3;
            self.expect_kw("ORDER")?;
            self.expect_kw("BY")?;
            within = Some(self.comma_list(|parser| parser.sort_item())?);
            self.expect_char(')')?;
        }
        let mut filter: Option<Expression> = None;
        if self.kw("FILTER") && self.char_at(1, '(') {
            self.at += 2;
            self.expect_kw("WHERE")?;
            filter = Some(self.expression()?);
            self.expect_char(')')?;
        }
        let mut ignore_nulls: Option<bool> = None;
        if self.kw_in(0, &["IGNORE", "RESPECT"]) && self.kw_at(1, "NULLS") {
            ignore_nulls = Some(self.kw("IGNORE"));
            self.at += 2;
        }
        let window: Option<Over> = match self.eat_kw("OVER") {
            true => Some(self.window_spec()?),
            false => None,
        };
        let call = function(name, args);
        let mut node: Expression = match distinct || filter.is_some() || ignore_nulls.is_some() {
            true => aggregate(call, distinct, filter, ignore_nulls),
            false => Expression::Function(Box::new(call)),
        };
        if let Some(order_by) = within {
            node = Expression::WithinGroup(Box::new(WithinGroup {
                this: node,
                order_by,
            }));
        }
        Ok(match window {
            Some(window) => window_function(node, window),
            None => node,
        })
    }

    /// `functionTableArgument`: a `TABLE` argument, a named argument or an expression.
    pub(crate) fn function_argument(&mut self) -> Parsing<Expression> {
        if self.kw("TABLE") && (self.char_at(1, '(') || self.is_at(1, is_identifier)) {
            return self.table_argument();
        }
        if self.is(is_identifier) && self.op_at(1, "=>") {
            let name: Identifier = self.name(is_identifier)?;
            self.at += 1;
            let value: Expression = match self.kw("TABLE") {
                true => self.table_argument()?,
                false => self.expression()?,
            };
            return Ok(Expression::NamedArgument(Box::new(NamedArgument {
                name,
                value,
                separator: NamedArgSeparator::DArrow,
            })));
        }
        self.expression()
    }

    /// `functionTableSubqueryArgument`, partitioning and ordering parsed and not kept.
    fn table_argument(&mut self) -> Parsing<Expression> {
        self.expect_kw("TABLE")?;
        let this: Expression = match self.is_char('(') {
            true => {
                self.at += 1;
                let inner: Expression = match self.starts_query_at(0) && !self.char_at(1, ')') {
                    true => subquery(self.query()?, None, false),
                    false => Expression::Table(Box::new(table(self.multipart_identifier()?))),
                };
                self.expect_char(')')?;
                inner
            }
            false => Expression::Table(Box::new(table(self.multipart_identifier()?))),
        };
        let start: usize = self.offset();
        let single: bool = self.eat_kws(&["WITH", "SINGLE", "PARTITION"]);
        let partitioned: bool =
            !single && self.kw_in(0, &["PARTITION", "DISTRIBUTE"]) && self.kw_at(1, "BY");
        if single || partitioned {
            if partitioned {
                self.at += 2;
                self.comma_list(|parser| parser.expression())?;
            }
            if self.kw_in(0, &["ORDER", "SORT"]) && self.kw_at(1, "BY") {
                self.at += 2;
                self.comma_list(|parser| parser.sort_item())?;
            }
            self.gap("table_argument_partitioning", start);
        }
        Ok(Expression::TableArgument(Box::new(TableArgument {
            prefix: "TABLE".to_owned(),
            this,
        })))
    }

    /// `windowSpec`: a window name, or `( [PARTITION|DISTRIBUTE|CLUSTER BY ...] [ORDER|SORT BY ...] [frame] )`.
    pub(crate) fn window_spec(&mut self) -> Parsing<Over> {
        if self.is(is_identifier) {
            let name: Identifier = self.ident()?;
            return Ok(Over {
                alias: Some(name.clone()),
                ..over(Some(name), Vec::new(), Vec::new(), None)
            });
        }
        self.expect_char('(')?;
        if self.is(is_identifier) && self.char_at(1, ')') {
            let name: Identifier = self.ident()?;
            self.at += 1;
            return Ok(over(Some(name), Vec::new(), Vec::new(), None));
        }
        let mut partition: Vec<Expression> = Vec::new();
        let mut order: Vec<Ordered> = Vec::new();
        if self.kw("CLUSTER") && self.kw_at(1, "BY") {
            self.at += 2;
            partition = self.comma_list(|parser| parser.expression())?;
        } else {
            if self.kw_in(0, &["PARTITION", "DISTRIBUTE"]) && self.kw_at(1, "BY") {
                self.at += 2;
                partition = self.comma_list(|parser| parser.expression())?;
            }
            if self.kw_in(0, &["ORDER", "SORT"]) && self.kw_at(1, "BY") {
                self.at += 2;
                order = self.comma_list(|parser| parser.sort_item())?;
            }
        }
        let frame: Option<WindowFrame> = self.window_frame()?;
        self.expect_char(')')?;
        Ok(over(None, partition, order, frame))
    }

    /// `windowFrame`: `RANGE|ROWS bound` or `RANGE|ROWS BETWEEN bound AND bound`.
    fn window_frame(&mut self) -> Parsing<Option<WindowFrame>> {
        let kind: WindowFrameKind = match self.word_at(0).as_deref() {
            Some("RANGE") => WindowFrameKind::Range,
            Some("ROWS") => WindowFrameKind::Rows,
            _ => return Ok(None),
        };
        let kind_text: String = self.advance()?.value.to_ascii_uppercase();
        let (start, end) = match self.eat_kw("BETWEEN") {
            true => {
                let start: WindowFrameBound = self.frame_bound()?;
                self.expect_kw("AND")?;
                (start, Some(self.frame_bound()?))
            }
            false => (self.frame_bound()?, None),
        };
        Ok(Some(WindowFrame {
            kind,
            start,
            end,
            exclude: None,
            kind_text: Some(kind_text),
            start_side_text: None,
            end_side_text: None,
        }))
    }

    /// `frameBound`.
    fn frame_bound(&mut self) -> Parsing<WindowFrameBound> {
        if self.kw("UNBOUNDED") && self.kw_in(1, &["PRECEDING", "FOLLOWING"]) {
            let preceding: bool = self.kw_at(1, "PRECEDING");
            self.at += 2;
            return Ok(match preceding {
                true => WindowFrameBound::UnboundedPreceding,
                false => WindowFrameBound::UnboundedFollowing,
            });
        }
        if self.kw("CURRENT") && self.kw_at(1, "ROW") {
            self.at += 2;
            return Ok(WindowFrameBound::CurrentRow);
        }
        let offset: Expression = self.expression()?;
        if self.eat_kw("PRECEDING") {
            return Ok(WindowFrameBound::Preceding(Box::new(offset)));
        }
        self.expect_kw("FOLLOWING")?;
        Ok(WindowFrameBound::Following(Box::new(offset)))
    }

    /// `sortItem`: `expression [ASC|DESC] [NULLS FIRST|LAST]`.
    pub(crate) fn sort_item(&mut self) -> Parsing<Ordered> {
        let this: Expression = self.expression()?;
        let (mut desc, mut explicit_asc) = (false, false);
        if self.eat_kw("ASC") {
            explicit_asc = true;
        } else if self.eat_kw("DESC") {
            desc = true;
        }
        let mut nulls_first: Option<bool> = None;
        if self.kw("NULLS") && self.kw_in(1, &["FIRST", "LAST"]) {
            nulls_first = Some(self.kw_at(1, "FIRST"));
            self.at += 2;
        }
        Ok(ordered(this, desc, explicit_asc, nulls_first))
    }

    /// `multipartIdentifierList` as joined names.
    pub(crate) fn name_parts_list(&mut self) -> Parsing<Vec<Identifier>> {
        self.comma_list(|parser| parser.multipart_identifier().map(joined))
    }
}
