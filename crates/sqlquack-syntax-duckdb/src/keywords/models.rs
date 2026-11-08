//! The keyword classes of a word.

/// The keyword classes of a word, as the grammar's keyword rules read them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeywordClasses {
    /// `ReservedKeyword`: never a bare column, table or function name.
    pub reserved: bool,
    /// `UnreservedKeyword`: a name anywhere.
    pub unreserved: bool,
    /// `ColumnNameKeyword`: a column or table name, not a function name.
    pub column_name: bool,
    /// `FuncNameKeyword`: a function or type name, not a column name.
    pub func_name: bool,
    /// `TypeNameKeyword`: a type name.
    pub type_name: bool,
    /// `BareLabelKeyword`: a column alias without `AS`.
    pub bare_label: bool,
}
