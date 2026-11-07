//! `parse_type`: a type name as PostgreSQL's `typeStringToTypeName` reads it.

use sqlquack_syntax::DataType;

use crate::parse::_helpers::tree::cursor::Parser;
use crate::parse::errors::ParseError;
use crate::scan::main::scan::scan;

/// The type `text` names, grammar only, as `typeStringToTypeName` reads it; SETOF gives 42601.
pub fn parse_type(text: &str) -> Result<DataType, ParseError> {
    let mut parser: Parser<'_> = Parser::new(text, scan(text));
    let setof: bool = parser.kw("setof");
    let data_type: DataType = parser.typename()?;
    if parser.peek().is_some() {
        return Err(parser.error());
    }
    if parser.opaque || setof {
        return Err(ParseError {
            message: format!("invalid type name \"{}\"", text.trim()),
            position: 0,
            syntax: true,
        });
    }
    Ok(data_type)
}
