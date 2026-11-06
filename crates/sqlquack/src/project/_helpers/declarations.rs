//! The tables and functions a project's folders declare, and the files declaring them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use sqlquack_policy::policy::models::ProjectPolicy;
use walkdir::WalkDir;

use crate::project::_helpers::function_files::{python_function, sql_function};
use crate::project::_helpers::source_files::source_relations;
use crate::project::constants::{
    FUNCTION_KIND, FUNCTIONS_FOLDER, PYTHON_EXTENSION, PYTHON_FUNCTIONS_FOLDER, SOURCE_EXTENSIONS,
    SOURCES_FOLDER, SQL_EXTENSION, SQL_FUNCTIONS_FOLDER, TABLE_KIND,
};
use crate::project::main::project_folder::project_folder;
use crate::project::models::Declaration;

/// Declarations as the schema file writes them, by name, and where each came from.
pub(crate) type Declared = (Map<String, Value>, Vec<Declaration>);

/// The tables of the project's source files.
pub(crate) fn source_tables(root: &Path, project: &ProjectPolicy) -> Result<Declared, String> {
    let (mut tables, mut declarations) = (Map::new(), Vec::new());
    let Some(folder) = project_folder(root, project.sources.as_deref(), SOURCES_FOLDER)? else {
        return Ok((tables, declarations));
    };
    for path in files(&folder, SOURCE_EXTENSIONS) {
        for (name, columns) in source_relations(&path)? {
            declarations.push(declaration(TABLE_KIND, &name, &path));
            tables.insert(name, columns);
        }
    }
    Ok((tables, declarations))
}

/// The functions of the project's SQL and Python function files, named after their files.
pub(crate) fn project_functions(root: &Path, project: &ProjectPolicy) -> Result<Declared, String> {
    let Some(folder) = project_folder(root, project.functions.as_deref(), FUNCTIONS_FOLDER)? else {
        return Ok((Map::new(), Vec::new()));
    };
    let (mut functions, mut declarations) = function_files(
        &folder.join(SQL_FUNCTIONS_FOLDER),
        SQL_EXTENSION,
        sql_function,
    )?;
    let (python, python_declarations) = function_files(
        &folder.join(PYTHON_FUNCTIONS_FOLDER),
        PYTHON_EXTENSION,
        python_function,
    )?;
    functions.extend(python);
    declarations.extend(python_declarations);
    Ok((functions, declarations))
}

/// The functions of the `extension` files in `folder`, each read by `read`.
fn function_files(
    folder: &Path,
    extension: &str,
    read: fn(&str) -> Result<Value, String>,
) -> Result<Declared, String> {
    let (mut functions, mut declarations) = (Map::new(), Vec::new());
    for path in files(folder, &[extension]) {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_ascii_uppercase();
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let declared = read(&text).map_err(|reason| format!("{}: {reason}", path.display()))?;
        declarations.push(declaration(FUNCTION_KIND, &name, &path));
        functions.insert(name, declared);
    }
    Ok((functions, declarations))
}

/// `name`, of `kind`, declared in `path`.
pub(crate) fn declaration(kind: &'static str, name: &str, path: &Path) -> Declaration {
    Declaration {
        kind,
        name: name.to_owned(),
        path: path.to_path_buf(),
    }
}

/// An error naming both files when two declare one kind and name, ignoring case.
pub(crate) fn no_clash(declarations: &[Declaration]) -> Result<(), String> {
    let mut seen: BTreeMap<(&str, String), &Declaration> = BTreeMap::new();
    for declared in declarations {
        let key = (declared.kind, declared.name.to_ascii_uppercase());
        if let Some(earlier) = seen.insert(key, declared) {
            return Err(format!(
                "the {} {} is declared twice, in {} and in {}; keep one",
                declared.kind,
                declared.name,
                earlier.path.display(),
                declared.path.display()
            ));
        }
    }
    Ok(())
}

/// The files under `folder` with one of `extensions`, in path order.
fn files(folder: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(folder)
        .into_iter()
        .filter_map(Result::ok)
        .map(walkdir::DirEntry::into_path)
        .filter(|file| {
            file.is_file()
                && file
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extensions.contains(&extension))
        })
        .collect();
    files.sort();
    files
}
