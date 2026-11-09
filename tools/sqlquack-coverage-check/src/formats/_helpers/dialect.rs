//! The current thread's dialect, Snowflake's defaults, and the checks of a dialect file.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;

use crate::formats::_helpers::files::read_toml;
use crate::formats::_helpers::ids::is_segment;
use crate::formats::constants::{
    DIALECT_FILE, DIALECT_FORMAT, ENGINE_CODE_LENGTH, FAMILIES, FORM_PREFIX, RECEIPT_FORMAT,
    RECEIPT_FORMAT_ENGINE, SNOWFLAKE_DIALECT, SNOWFLAKE_PARSER_PATH,
};
use crate::formats::models::{Dialect, LocalParser};

thread_local! {
    /// The dialect loading and checking on this thread follow; Snowflake's until a root sets it.
    static CURRENT: RefCell<Rc<Dialect>> = RefCell::new(Rc::new(snowflake()));
}

/// The settings of a root without a dialect file: exactly the Snowflake constants.
pub(crate) fn snowflake() -> Dialect {
    Dialect {
        format: DIALECT_FORMAT.to_owned(),
        name: SNOWFLAKE_DIALECT.to_owned(),
        form_prefix: FORM_PREFIX.to_owned(),
        families: FAMILIES.iter().map(|family| (*family).to_owned()).collect(),
        receipt_format: RECEIPT_FORMAT.to_owned(),
        parser_path: SNOWFLAKE_PARSER_PATH.to_owned(),
        local_parser: LocalParser::Snowflake,
        syntax_codes: Vec::new(),
    }
}

/// The dialect this thread loads and checks under.
pub(crate) fn current_dialect() -> Rc<Dialect> {
    CURRENT.with(|current| Rc::clone(&current.borrow()))
}

/// Makes `dialect` the one this thread loads and checks under.
pub(crate) fn set_dialect(dialect: Dialect) {
    CURRENT.with(|current| *current.borrow_mut() = Rc::new(dialect));
}

/// Whether `family` is one of the current dialect's.
pub(crate) fn known_family(family: &str) -> bool {
    current_dialect()
        .families
        .iter()
        .any(|known| known == family)
}

/// Five digits or upper-case letters, as a SQLSTATE.
fn is_engine_code(code: &str) -> bool {
    code.len() == ENGINE_CODE_LENGTH
        && code
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
}

/// Families that are distinct and known, at least one.
fn valid_families(families: &[String]) -> bool {
    let distinct: BTreeSet<&String> = families.iter().collect();
    !families.is_empty()
        && distinct.len() == families.len()
        && families
            .iter()
            .all(|family| FAMILIES.contains(&family.as_str()))
}

/// Every problem in a dialect file.
fn check_dialect(dialect: &Dialect) -> Vec<String> {
    let engine: bool = dialect.receipt_format == RECEIPT_FORMAT_ENGINE;
    let faults: [(bool, String); 6] = [
        (
            dialect.format != DIALECT_FORMAT,
            format!("format is not {DIALECT_FORMAT}"),
        ),
        (
            !is_segment(&dialect.name) || !is_segment(&dialect.form_prefix),
            "name and form_prefix are lower-case names".to_owned(),
        ),
        (
            !valid_families(&dialect.families),
            "families are distinct known families, at least one".to_owned(),
        ),
        (
            ![RECEIPT_FORMAT, RECEIPT_FORMAT_ENGINE].contains(&dialect.receipt_format.as_str()),
            format!("receipt_format is {RECEIPT_FORMAT} or {RECEIPT_FORMAT_ENGINE}"),
        ),
        (
            dialect.parser_path.trim().is_empty() || dialect.parser_path.starts_with('/'),
            "parser_path is a path relative to the repository".to_owned(),
        ),
        (
            engine == dialect.syntax_codes.is_empty()
                || !dialect.syntax_codes.iter().all(|code| is_engine_code(code)),
            "syntax_codes are five-character codes, required with receipt format /2 only"
                .to_owned(),
        ),
    ];
    faults
        .into_iter()
        .filter(|(fault, _)| *fault)
        .map(|(_, problem)| format!("{DIALECT_FILE}: {problem}"))
        .collect()
}

/// The root's `dialect.toml`, Snowflake's settings when it has none, or every problem in it.
pub(crate) fn load_dialect(root: &Path) -> Result<Dialect, Vec<String>> {
    let path = root.join(DIALECT_FILE);
    if !path.exists() {
        return Ok(snowflake());
    }
    let (dialect, _): (Dialect, String) = read_toml(&path).map_err(|problem| vec![problem])?;
    let problems: Vec<String> = check_dialect(&dialect);
    match problems.is_empty() {
        true => Ok(dialect),
        false => Err(problems),
    }
}
