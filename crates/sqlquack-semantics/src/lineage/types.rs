//! What lineage asks of a schema.

use std::collections::BTreeSet;
use std::rc::Rc;

use sqlquack_syntax::{DataType, Expression};

use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::{Engine, Nullability, OutputReach};
use crate::lineage::models::{
    Lineage, LineageContext, NullabilityRule, SchemaColumn, SourceColumn,
};

/// The known relations: each one's columns in order, by the relation's resolved name.
pub trait LineageSchema: std::fmt::Debug + Sync {
    /// The columns of `relation` (its parts resolved and joined by dots), when known.
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]>;

    /// Known columns of `relation` whose full column list is not known, to type the ones read.
    fn open_columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        let _ = relation;
        None
    }

    /// The type the function `name` returns whatever its arguments, when the catalog knows it.
    fn function_type(&self, name: &str) -> Option<&str> {
        let _ = name;
        None
    }

    /// How the function `name`'s nullability follows from its arguments', when the catalog knows.
    fn function_nullability(&self, name: &str) -> Option<NullabilityRule> {
        let _ = name;
        None
    }

    /// Whether the project declares the function `name`, whose calls the type checks leave alone.
    fn known_function(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// The `(least, most)` argument counts the catalog accepts for `name`, when it knows them.
    fn function_arities(&self, name: &str, canonical: &str) -> Option<Vec<(usize, Option<usize>)>> {
        let _ = (name, canonical);
        None
    }

    /// Whether the project declares the type `name`, which a cast may name.
    fn known_type(&self, name: &str) -> bool {
        let _ = name;
        false
    }
}

/// A set of source columns that copies of a value's lineage share until one changes it.
pub(crate) type SharedColumns = Rc<BTreeSet<SourceColumn>>;

/// How one dialect types values, names outputs and knows its functions, beyond the shared rules.
pub(crate) trait DialectRules: Sync {
    /// The engine whose binder the coercion checks follow for this dialect.
    fn coercion_engine(&self) -> Engine;

    /// Whether lineage keeps to the name rules, leaving out the scope, type and set-operation ones.
    fn checks_names_only(&self) -> bool;

    /// Which output names of a SELECT its own clauses may read.
    fn output_reach(&self) -> OutputReach;

    /// Whether calls are checked against the signatures of the functions they name.
    fn checks_call_signatures(&self) -> bool;

    /// Whether set operations check only that their branches return as many columns.
    fn checks_set_widths_only(&self) -> bool;

    /// A literal's type and nullability, given the shared rules' `generic` answer.
    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        let _ = expression;
        generic
    }

    /// A computed value's type and nullability, given the shared rules' `generic` answer.
    fn computed_type(
        &self,
        expression: &Expression,
        parts: &[(&Expression, Lineage)],
        context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        let _ = (expression, parts, context);
        generic
    }

    /// The one type operands of differing types unify to, when the dialect unifies them.
    fn unified_type(&self, operands: &[&Lineage]) -> Option<Rc<str>> {
        let _ = operands;
        None
    }

    /// The type alternative branches of differing types combine into, when the dialect knows it.
    fn merged_type(&self, typed: &[&Lineage]) -> Option<Rc<str>> {
        let _ = typed;
        None
    }

    /// A set operation's type, from its `merged` type, its typed `branches` and all its `parts`.
    fn set_type(
        &self,
        merged: Option<Rc<str>>,
        branches: &[Lineage],
        parts: &[Lineage],
    ) -> Option<Rc<str>> {
        let _ = (branches, parts);
        merged
    }

    /// A cast target's spelling, when lineage knows the type.
    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        rendered_type(data_type)
    }

    /// The name the dialect gives an unaliased output that is not a column, when it names it.
    fn unaliased_name(&self, projection: &Expression) -> Option<String> {
        let _ = projection;
        None
    }

    /// Whether `expression` calls a built-in function the dialect runs only with OVER.
    fn window_only(&self, expression: &Expression) -> bool {
        let _ = expression;
        false
    }

    /// The `(least, most)` argument counts of the built-in function `name`, when recorded.
    fn builtin_arities(&self, name: &str) -> Option<Vec<(usize, Option<usize>)>> {
        let _ = name;
        None
    }

    /// Whether an unqualified call to `name` names no built-in function the dialect records.
    fn unknown_function(&self, name: &str) -> bool {
        let _ = name;
        false
    }
}

/// A value's type and nullability.
pub(crate) type ValueType = (Option<Rc<str>>, Nullability);
