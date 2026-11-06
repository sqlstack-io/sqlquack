//! `parse_type`: a type name as PostgreSQL's `typeStringToTypeName` reads it.

use sqlquack_syntax::DataType;

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;

/// The type `text` names, such as `numeric(18, 2)` or `pg_catalog.varchar(5)[]`.
///
/// Only the grammar is checked, as PostgreSQL's raw parser does: an unknown name, or a modifier
/// the type refuses (`int4(3)`, `varchar(0)`), parses and fails only when the type is looked up.
/// `SETOF` is refused with 42601, as PostgreSQL refuses it in a type string.
pub fn parse_type(text: &str) -> Result<DataType, ParseError> {
    let mut parser: Parser<'_> = Parser::new(text, scan(text));
    let data_type: DataType = parser.typename()?;
    if parser.peek().is_some() {
        return Err(parser.error());
    }
    if parser.opaque {
        return Err(ParseError {
            message: format!("invalid type name \"{}\"", text.trim()),
            position: 0,
            syntax: true,
        });
    }
    Ok(data_type)
}
