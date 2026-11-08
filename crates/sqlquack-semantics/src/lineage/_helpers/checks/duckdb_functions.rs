//! DuckDB's built-in functions as the engine lists them: which names exist and their argument counts.

use std::collections::HashMap;
use std::sync::LazyLock;

/// `duckdb_functions()` of DuckDB 2.0.0 with its loadable extensions; see `manifest.toml`.
const FUNCTIONS: &str = include_str!("duckdb_catalog/functions.csv");
/// How the catalogue writes a variadic form's flag.
const VARIADIC: &str = "true";

/// A name's `(least, most)` argument counts; none when one of its forms does not record them.
type Arities = Option<Vec<(usize, Option<usize>)>>;

static CATALOG: LazyLock<HashMap<&'static str, Arities>> = LazyLock::new(catalog);

/// Read `name,kind,arguments,variadic` rows into each name's argument counts.
fn catalog() -> HashMap<&'static str, Arities> {
    let mut names: HashMap<&'static str, Arities> = HashMap::new();
    for line in FUNCTIONS.lines().skip(1) {
        let mut fields = line.split(',');
        let (Some(name), Some(_kind), Some(arguments), Some(variadic)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let form: Option<(usize, Option<usize>)> = match arguments.parse::<usize>() {
            Ok(count) => Some((count, (variadic != VARIADIC).then_some(count))),
            Err(_) => None,
        };
        let entry: &mut Arities = names.entry(name).or_insert_with(|| Some(Vec::new()));
        match (entry.as_mut(), form) {
            (Some(forms), Some(form)) if !forms.contains(&form) => forms.push(form),
            (Some(_), None) => *entry = None,
            _ => {}
        }
    }
    for forms in names.values_mut().flatten() {
        forms.sort_unstable();
    }
    names
}

/// The argument counts DuckDB accepts for the unqualified built-in `name`, when it records them.
pub(crate) fn duckdb_arities(name: &str) -> Option<Vec<(usize, Option<usize>)>> {
    CATALOG.get(name.to_lowercase().as_str())?.clone()
}

/// Whether DuckDB has a built-in function, macro or special form called `name`.
pub(crate) fn duckdb_function(name: &str) -> bool {
    CATALOG.contains_key(name.to_lowercase().as_str())
}
