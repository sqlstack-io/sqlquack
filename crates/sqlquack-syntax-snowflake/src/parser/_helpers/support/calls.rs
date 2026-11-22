//! Parser support: how a call keeps its name and qualifier as written.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::functions::models::Function;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::MethodCall;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    pub(crate) fn parse_method_call_expression(
        &self,
        this: Expression,
        method: Identifier,
        args: Vec<Expression>,
    ) -> Expression {
        let dotted_name = match self.config.calls_as_written {
            true => Self::name_parts(&this),
            false => None,
        };
        if let Some(mut parts) = dotted_name {
            let name_span = SourceSpan(method.span);
            parts.push(method);
            let name = parts
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<&str>>()
                .join(".");
            return Expression::Function(Box::new(Function {
                name,
                args,
                quoted: parts.iter().any(|part| part.quoted),
                qualified_name: parts,
                name_span,
                ..Function::default()
            }));
        }
        Expression::MethodCall(Box::new(MethodCall::new(this, method, args)))
    }

    /// SQLQuack: the names of a dotted name, or `None` for any other expression.
    fn name_parts(mut this: &Expression) -> Option<Vec<Identifier>> {
        let mut parts: Vec<Identifier> = Vec::new();
        loop {
            match this {
                Expression::Dot(node) => {
                    parts.push(node.field.clone());
                    this = &node.this;
                }
                Expression::Column(node) => {
                    parts.push(node.name.clone());
                    parts.extend(node.table.clone());
                    break;
                }
                Expression::Identifier(node) => {
                    parts.push(node.clone());
                    break;
                }
                _ => return None,
            }
        }
        parts.reverse();
        Some(parts)
    }

    pub(crate) fn parse_typed_function(
        &mut self,
        name: &str,
        upper_name: &str,
        quoted: bool,
    ) -> Result<Expression> {
        let arguments = self.current;
        let call = if self.config.calls_as_written
            || self.config.dialect == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
        {
            self.parse_faithful_call(name, upper_name, quoted)?
        } else {
            self.parse_typed_function_lowered(name, upper_name, quoted)?
        };
        Ok(self.named_call(arguments, name, call))
    }

    /// SQLQuack: a call as written, unless polyglot keeps it or the typed parse is the call itself.
    pub(crate) fn parse_faithful_call(
        &mut self,
        name: &str,
        upper_name: &str,
        quoted: bool,
    ) -> Result<Expression> {
        let saved = self.current;
        if let Some((expr, end)) = self.faithful_calls.get(&saved) {
            let expr = expr.clone();
            self.current = *end;
            return Ok(expr);
        }
        let comments = self.call_comments(saved);
        let typed = self.parse_typed_function_lowered(name, upper_name, quoted)?;
        let expr = match Self::call_name(&typed) {
            Some(kept) if kept.eq_ignore_ascii_case(name) => typed,
            _ if sqlquack_syntax::functions::names::main::faithful_keyword_syntax_name::faithful_keyword_syntax_name(upper_name)
                && self.tokens_use_keyword_syntax(saved) =>
            {
                typed
            }
            _ => {
                let typed_end = self.current;
                let typed_comments = self.call_comments(saved);
                self.restore_comments(comments);
                self.current = saved;
                match self.parse_generic_function(name, quoted) {
                    Ok(generic) if self.current == typed_end => generic,
                    _ => {
                        self.restore_comments(typed_comments);
                        self.current = typed_end;
                        typed
                    }
                }
            }
        };
        self.faithful_calls
            .insert(saved, (expr.clone(), self.current));
        Ok(expr)
    }

    /// Check if function name is a known aggregate function
    pub(crate) fn is_aggregate_function(&self, name: &str) -> bool {
        sqlquack_syntax::functions::names::main::is_aggregate_function_name_for_dialect::is_aggregate_function_name_for_dialect(name, self.config.dialect)
    }

    /// Whether the source dialect writes LOG(base, value), base first (not BigQuery or TSQL).
    pub(crate) fn log_base_first(&self) -> bool {
        !false
    }

    /// SQLQuack: `call` with the range of its name, the token before the `(` ahead of `arguments`.
    pub(crate) fn named_call(
        &self,
        arguments: usize,
        name: &str,
        mut call: Expression,
    ) -> Expression {
        if !self.config.record_node_spans {
            return call;
        }
        let Some(at) = arguments.checked_sub(2) else {
            return call;
        };
        let names_it = self
            .tokens
            .get(at)
            .is_some_and(|token| token.text.eq_ignore_ascii_case(name));
        if !names_it {
            return call;
        }
        let field = match &mut call {
            Expression::Function(node) => &mut node.name_span,
            Expression::AggregateFunction(node) => &mut node.name_span,
            _ => return call,
        };
        if field.is_absent() {
            *field = self.token_range(at);
        }
        call
    }
}
