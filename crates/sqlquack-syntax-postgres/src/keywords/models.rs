//! A PostgreSQL keyword and its grammar category.

/// Where the grammar lets a keyword stand as a name (gram.y's keyword categories).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordCategory {
    /// A column, table, function or type name anywhere (`unreserved_keyword`).
    Unreserved,
    /// A column or table name, not a function or type name (`col_name_keyword`).
    ColName,
    /// A function or type name, not a column or table name (`type_func_name_keyword`).
    TypeFuncName,
    /// Never a name, only a label after `AS` (`reserved_keyword`).
    Reserved,
}

/// One row of kwlist.h.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyword {
    /// The keyword in lower case.
    pub name: &'static str,
    pub category: KeywordCategory,
    /// Whether it may label a select-list item without `AS` (`BARE_LABEL`).
    pub bare_label: bool,
}

impl Keyword {
    /// A kwlist.h row.
    pub(crate) const fn new(
        name: &'static str,
        category: KeywordCategory,
        bare_label: bool,
    ) -> Self {
        Self {
            name,
            category,
            bare_label,
        }
    }
}
