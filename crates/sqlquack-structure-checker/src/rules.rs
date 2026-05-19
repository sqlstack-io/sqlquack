use crate::models;
use std::fs;
use std::path;
use syn::visit;
use syn::visit::Visit;

const MAX_LIB_RS_LINES: usize = 80;

pub fn check_repo(repo_root: &path::Path) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    let src_root = repo_root.join("crates/sqlquack/src");

    if src_root.exists() {
        for entry in walkdir::WalkDir::new(&src_root) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }

            let source = fs::read_to_string(path).unwrap_or_default();
            let syntax = syn::parse_file(&source).ok();

            violations.extend(check_banned_names(repo_root, path));
            violations.extend(check_lib_rs_size(repo_root, path, &source));
            if let Some(file) = syntax.as_ref() {
                violations.extend(check_mod_rs_shape(path, file));
                violations.extend(check_role_file_shape(path, file));
                violations.extend(check_main_entry_shape(repo_root, path, file));
                violations.extend(check_helper_file_shape(repo_root, path, file));
                violations.extend(check_reexports(repo_root, path, file));
                violations.extend(check_imports(repo_root, path, file));
            }
            violations.extend(check_sqlbuild_terms(path, &source));
        }
    }

    violations.extend(check_test_structure(repo_root));

    violations.extend(check_crate_dependency_direction(repo_root));
    violations
}

fn check_banned_names(repo_root: &path::Path, file_path: &path::Path) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    let relative = relative_parts(repo_root, file_path);

    if matches!(
        file_path.file_name().and_then(|value| value.to_str()),
        Some("utils.rs" | "misc.rs" | "common.rs")
    ) {
        violations.push(models::Violation::new(
            "SQS001",
            file_path,
            None,
            "uses banned generic filename",
        ));
    }
    for part in &relative {
        if matches!(part.as_str(), "utils" | "misc" | "common") {
            violations.push(models::Violation::new(
                "SQS002",
                file_path,
                None,
                format!("is under banned generic directory '{part}'"),
            ));
        }
    }
    if file_path.file_name().and_then(|value| value.to_str()) == Some("helpers.rs") {
        violations.push(models::Violation::new(
            "SQS003",
            file_path,
            None,
            "use a scoped helpers/ folder instead of helpers.rs",
        ));
    }

    violations
}

fn check_lib_rs_size(
    repo_root: &path::Path,
    file_path: &path::Path,
    source: &str,
) -> Vec<models::Violation> {
    if relative_path(repo_root, file_path).as_deref() != Some("crates/sqlquack/src/lib.rs") {
        return Vec::new();
    }

    let line_count = source.lines().count();
    if line_count <= MAX_LIB_RS_LINES {
        return Vec::new();
    }

    vec![models::Violation::new(
        "SQS004",
        file_path,
        None,
        format!("lib.rs has {line_count} lines; keep it at or below {MAX_LIB_RS_LINES}"),
    )]
}

fn check_mod_rs_shape(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    if file_path.file_name().and_then(|value| value.to_str()) != Some("mod.rs") {
        return Vec::new();
    }

    file.items
        .iter()
        .filter(|item| !matches!(item, syn::Item::Mod(_) | syn::Item::Use(_)))
        .map(|item| {
            models::Violation::new(
                "SQS012",
                file_path,
                item_line(item),
                "mod.rs must contain declarations only, not implementation items",
            )
        })
        .collect()
}

fn check_role_file_shape(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    match file_path.file_name().and_then(|value| value.to_str()) {
        Some("models.rs") => check_models_rs_shape(file_path, file),
        Some("types.rs") => check_types_rs_shape(file_path, file),
        Some("constants.rs") => check_constants_rs_shape(file_path, file),
        _ => Vec::new(),
    }
}

fn check_models_rs_shape(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Fn(item_fn) => violations.push(models::Violation::new(
                "SQS013",
                file_path,
                Some(item_fn.sig.ident.span().start().line),
                "models.rs must not define free functions",
            )),
            syn::Item::Impl(item_impl) => {
                for impl_item in &item_impl.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        if matches!(method.vis, syn::Visibility::Public(_)) {
                            violations.push(models::Violation::new(
                                "SQS014",
                                file_path,
                                Some(method.sig.ident.span().start().line),
                                "models.rs must not expose public methods; use pub(crate) or narrower",
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    violations
}

fn check_types_rs_shape(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    file.items
        .iter()
        .filter(|item| {
            !matches!(
                item,
                syn::Item::Use(_) | syn::Item::Type(_) | syn::Item::Trait(_)
            )
        })
        .map(|item| {
            models::Violation::new(
                "SQS015",
                file_path,
                item_line(item),
                "types.rs must stay narrow: type aliases and traits only",
            )
        })
        .collect()
}

fn check_constants_rs_shape(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    file.items
        .iter()
        .filter(|item| !matches!(item, syn::Item::Use(_) | syn::Item::Const(_)))
        .map(|item| {
            models::Violation::new(
                "SQS016",
                file_path,
                item_line(item),
                "constants.rs must contain constants only",
            )
        })
        .collect()
}

fn check_main_entry_shape(
    repo_root: &path::Path,
    file_path: &path::Path,
    file: &syn::File,
) -> Vec<models::Violation> {
    let relative = relative_parts(repo_root, file_path);
    if !relative.iter().any(|part| part == "main")
        || file_path.file_name().and_then(|value| value.to_str()) == Some("mod.rs")
    {
        return Vec::new();
    }

    let mut visible_functions = 0;
    let mut private_functions = 0;
    let mut violations = Vec::new();

    for item in &file.items {
        match item {
            syn::Item::Fn(item_fn) => {
                if matches!(item_fn.vis, syn::Visibility::Inherited) {
                    private_functions += 1;
                } else {
                    visible_functions += 1;
                }
            }
            syn::Item::Use(_) => {}
            _ => violations.push(models::Violation::new(
                "SQS017",
                file_path,
                item_line(item),
                "main/ entry files must contain orchestration functions only",
            )),
        }
    }

    if visible_functions != 1 {
        violations.push(models::Violation::new(
            "SQS018",
            file_path,
            None,
            "main/ entry files must define exactly one visible entry function",
        ));
    }
    if private_functions > 2 {
        violations.push(models::Violation::new(
            "SQS019",
            file_path,
            None,
            "main/ entry files must define at most two private helper functions",
        ));
    }

    violations
}

fn check_helper_file_shape(
    repo_root: &path::Path,
    file_path: &path::Path,
    file: &syn::File,
) -> Vec<models::Violation> {
    let relative = relative_parts(repo_root, file_path);
    if !relative.iter().any(|part| part == "helpers") {
        return Vec::new();
    }

    let mut violations = Vec::new();
    if file_path.file_name().and_then(|value| value.to_str()) == Some("main.rs") {
        violations.push(models::Violation::new(
            "SQS020",
            file_path,
            None,
            "helpers/ must not contain main.rs",
        ));
    }

    for item in &file.items {
        if is_public_item(item) {
            violations.push(models::Violation::new(
                "SQS021",
                file_path,
                item_line(item),
                "helpers/ must not expose public API; use pub(super) or private items",
            ));
        }
    }

    violations
}

fn check_reexports(
    repo_root: &path::Path,
    file_path: &path::Path,
    file: &syn::File,
) -> Vec<models::Violation> {
    let is_root_lib =
        relative_path(repo_root, file_path).as_deref() == Some("crates/sqlquack/src/lib.rs");
    let mut violations = Vec::new();

    for item in &file.items {
        let syn::Item::Use(item_use) = item else {
            continue;
        };
        if contains_wildcard(&item_use.tree) {
            violations.push(models::Violation::new(
                "SQS007",
                file_path,
                Some(item_use.use_token.span.start().line),
                "wildcard imports are not allowed",
            ));
        }
        match item_use.vis {
            syn::Visibility::Public(_) if !is_root_lib => violations.push(models::Violation::new(
                "SQS022",
                file_path,
                Some(item_use.use_token.span.start().line),
                "pub use is only allowed in lib.rs unless explicitly allowlisted",
            )),
            syn::Visibility::Restricted(_) => violations.push(models::Violation::new(
                "SQS023",
                file_path,
                Some(item_use.use_token.span.start().line),
                "pub(crate) use re-exports are banned; import role modules explicitly",
            )),
            _ => {}
        }
    }

    violations
}

fn check_imports(
    repo_root: &path::Path,
    file_path: &path::Path,
    file: &syn::File,
) -> Vec<models::Violation> {
    let Some(module_name) = top_sqlquack_module(repo_root, file_path) else {
        return Vec::new();
    };

    let mut violations = Vec::new();
    for item in &file.items {
        let syn::Item::Use(item_use) = item else {
            continue;
        };
        for path_parts in use_tree_paths(&item_use.tree) {
            violations.extend(check_forbidden_import_path(
                file_path,
                &module_name,
                &path_parts,
                item_use,
            ));
            violations.extend(check_cross_module_helper_import_path(
                file_path,
                &module_name,
                &path_parts,
                item_use,
            ));
            violations.extend(check_helper_model_import_path(
                repo_root,
                file_path,
                &path_parts,
                item_use,
            ));
        }
    }

    violations
}

fn check_forbidden_import_path(
    file_path: &path::Path,
    module_name: &str,
    path_parts: &[String],
    item_use: &syn::ItemUse,
) -> Vec<models::Violation> {
    if path_parts.first().map(String::as_str) != Some("crate") || path_parts.len() < 2 {
        return Vec::new();
    }
    let imported = path_parts[1].as_str();
    let forbidden = match module_name {
        "ast" => &["parser", "analysis", "lineage", "translation"][..],
        "lexer" => &["parser", "analysis", "lineage", "translation"][..],
        "parser" => &["analysis", "lineage", "translation"][..],
        "analysis" => &["translation"][..],
        "lineage" => &["translation"][..],
        "types" => &["translation"][..],
        "translation" => &["parser"][..],
        _ => &[],
    };
    if forbidden.contains(&imported) {
        return vec![models::Violation::new(
            "SQS005",
            file_path,
            Some(item_use.use_token.span.start().line),
            format!("{module_name}/ must not import downstream module crate::{imported}"),
        )];
    }
    Vec::new()
}

fn check_cross_module_helper_import_path(
    file_path: &path::Path,
    module_name: &str,
    path_parts: &[String],
    item_use: &syn::ItemUse,
) -> Vec<models::Violation> {
    if path_parts.first().map(String::as_str) != Some("crate") || path_parts.len() < 3 {
        return Vec::new();
    }
    let imported_module = &path_parts[1];
    if path_parts[2] == "helpers" && imported_module != module_name {
        return vec![models::Violation::new(
            "SQS011",
            file_path,
            Some(item_use.use_token.span.start().line),
            format!(
                "{module_name}/ must not import internal helpers from crate::{imported_module}; use an approved parent/main/public surface"
            ),
        )];
    }
    Vec::new()
}

fn check_helper_model_import_path(
    repo_root: &path::Path,
    file_path: &path::Path,
    path_parts: &[String],
    item_use: &syn::ItemUse,
) -> Vec<models::Violation> {
    if !relative_parts(repo_root, file_path)
        .iter()
        .any(|part| part == "helpers")
    {
        return Vec::new();
    }
    if path_parts.first().map(String::as_str) != Some("super")
        || path_parts.get(1).map(String::as_str) != Some("models")
    {
        return Vec::new();
    }
    if path_parts.len() > 2 || contains_wildcard(&item_use.tree) {
        return vec![models::Violation::new(
            "SQS009",
            file_path,
            Some(item_use.use_token.span.start().line),
            "helpers should use `use super::models;` and qualified model names",
        )];
    }
    Vec::new()
}

fn check_sqlbuild_terms(file_path: &path::Path, source: &str) -> Vec<models::Violation> {
    let terms = [
        "SqlReferenceKind",
        "MODEL(",
        "TEST(",
        "SCENARIO(",
        "AUDIT(",
        "SQLBuild",
        "sqlbuild",
    ];
    let mut violations = Vec::new();
    for (index, line) in source.lines().enumerate() {
        for term in terms {
            if line.contains(term) {
                violations.push(models::Violation::new(
                    "SQS006",
                    file_path,
                    Some(index + 1),
                    format!("SQLBuild-specific term '{term}' must not appear in core"),
                ));
            }
        }
    }
    violations
}

fn check_crate_dependency_direction(repo_root: &path::Path) -> Vec<models::Violation> {
    let manifest = repo_root.join("crates/sqlquack/Cargo.toml");
    let Ok(text) = fs::read_to_string(&manifest) else {
        return Vec::new();
    };
    if text.contains("sqlquack-python") || text.contains("sqlquack_python") {
        return vec![models::Violation::new(
            "SQS010",
            &manifest,
            None,
            "sqlquack core must not depend on sqlquack-python",
        )];
    }
    Vec::new()
}

fn check_test_structure(repo_root: &path::Path) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    let crates_root = repo_root.join("crates");
    if !crates_root.exists() {
        return violations;
    }

    for crate_entry in walkdir::WalkDir::new(&crates_root).max_depth(1) {
        let Ok(crate_entry) = crate_entry else {
            continue;
        };
        if !crate_entry.path().is_dir() || crate_entry.path() == crates_root {
            continue;
        }

        let tests_root = crate_entry.path().join("tests");
        if !tests_root.exists() {
            continue;
        }

        violations.extend(check_crate_tests_root(&tests_root));
    }

    violations
}

fn check_crate_tests_root(tests_root: &path::Path) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    let mut harness_stems = Vec::new();

    let Ok(entries) = fs::read_dir(tests_root) else {
        return violations;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("rs") {
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            harness_stems.push(stem.to_owned());
            violations.extend(check_test_harness_file(&path, stem));
        } else if path.is_dir() {
            violations.extend(check_test_area_folder(&path));
        }
    }

    for stem in harness_stems {
        let area_dir = tests_root.join(&stem);
        if !area_dir.is_dir() {
            violations.push(models::Violation::new(
                "SQT001",
                &tests_root.join(format!("{stem}.rs")),
                None,
                "test harness files must have a sibling area folder with the same stem",
            ));
            continue;
        }
        for required_file in ["helpers.rs", "test_types.rs"] {
            let required_path = area_dir.join(required_file);
            if !required_path.is_file() {
                violations.push(models::Violation::new(
                    "SQT002",
                    &area_dir,
                    None,
                    format!("test area folder must contain {required_file}"),
                ));
            }
        }
        violations.extend(check_harness_declares_area_files(
            &tests_root.join(format!("{stem}.rs")),
            &area_dir,
        ));
    }

    violations
}

fn check_harness_declares_area_files(
    harness_path: &path::Path,
    area_dir: &path::Path,
) -> Vec<models::Violation> {
    let harness_source = fs::read_to_string(harness_path).unwrap_or_default();
    let Ok(harness_file) = syn::parse_file(&harness_source) else {
        return Vec::new();
    };
    let area_name = area_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    let mut declared_files = Vec::new();
    let mut violations = Vec::new();
    for item in &harness_file.items {
        let syn::Item::Mod(item_mod) = item else {
            continue;
        };
        let module_name = item_mod.ident.to_string();
        let declared_file = format!("{module_name}.rs");
        declared_files.push(declared_file.clone());

        let expected_path_fragment = format!("{area_name}/{declared_file}");
        if !harness_source.contains(&expected_path_fragment) {
            continue;
        }
        if !area_dir.join(&declared_file).is_file() {
            violations.push(models::Violation::new(
                "SQT019",
                harness_path,
                Some(item_mod.ident.span().start().line),
                "test harness declares a module whose area file does not exist",
            ));
        }
    }

    let Ok(entries) = fs::read_dir(area_dir) else {
        return violations;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if matches!(file_name, "helpers.rs" | "test_types.rs") {
            continue;
        }
        if !declared_files.iter().any(|declared| declared == file_name) {
            violations.push(models::Violation::new(
                "SQT020",
                &path,
                None,
                "test area files must be declared by their top-level harness",
            ));
        }
    }

    violations
}

fn check_test_area_folder(folder_path: &path::Path) -> Vec<models::Violation> {
    let mut violations = Vec::new();

    let Ok(entries) = fs::read_dir(folder_path) else {
        return violations;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            violations.push(models::Violation::new(
                "SQT003",
                &path,
                None,
                "test area folders must stay flat",
            ));
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            violations.push(models::Violation::new(
                "SQT004",
                &path,
                None,
                "test area folders may contain only Rust files",
            ));
            continue;
        }

        let source = fs::read_to_string(&path).unwrap_or_default();
        let syntax = syn::parse_file(&source).ok();
        if let Some(file) = syntax.as_ref() {
            match file_name {
                "helpers.rs" => violations.extend(check_test_helpers_file(&path, file)),
                "test_types.rs" => violations.extend(check_test_types_file(&path, file)),
                _ => violations.extend(check_primary_test_module_file(&path, file)),
            }
        }
    }

    violations
}

fn check_test_harness_file(file_path: &path::Path, stem: &str) -> Vec<models::Violation> {
    let source = fs::read_to_string(file_path).unwrap_or_default();
    let Ok(file) = syn::parse_file(&source) else {
        return Vec::new();
    };

    let mut violations = Vec::new();

    for item in &file.items {
        match item {
            syn::Item::Mod(item_mod) => {
                if item_mod.content.is_some() {
                    violations.push(models::Violation::new(
                        "SQT005",
                        file_path,
                        Some(item_mod.ident.span().start().line),
                        "test harness module declarations must point at files, not inline modules",
                    ));
                }
                let module_name = item_mod.ident.to_string();
                let expected_path_fragment = format!("{stem}/{module_name}.rs");
                if !source.contains(&expected_path_fragment) {
                    violations.push(models::Violation::new(
                        "SQT009",
                        file_path,
                        Some(item_mod.ident.span().start().line),
                        "test harness modules must use #[path = \"<area>/<module>.rs\"]",
                    ));
                }
            }
            _ => violations.push(models::Violation::new(
                "SQT008",
                file_path,
                item_line(item),
                "test harness files may contain only path-based module declarations",
            )),
        }
    }

    violations
}

fn check_primary_test_module_file(
    file_path: &path::Path,
    file: &syn::File,
) -> Vec<models::Violation> {
    let mut violations = Vec::new();

    for item in &file.items {
        match item {
            syn::Item::Use(item_use) => {
                if contains_wildcard(&item_use.tree) {
                    violations.push(models::Violation::new(
                        "SQT006",
                        file_path,
                        Some(item_use.use_token.span.start().line),
                        "test files must not use wildcard imports",
                    ));
                }
            }
            syn::Item::Const(_) => {}
            syn::Item::Fn(item_fn) => {
                if !is_test_fn(item_fn) {
                    violations.push(models::Violation::new(
                        "SQT007",
                        file_path,
                        Some(item_fn.sig.ident.span().start().line),
                        "primary test module files must not define helper functions",
                    ));
                    continue;
                }
                violations.extend(check_test_function_shape(file_path, item_fn));
            }
            _ => violations.push(models::Violation::new(
                "SQT008",
                file_path,
                item_line(item),
                "primary test module files may contain only imports, constants, and test functions",
            )),
        }
    }

    violations
}

fn check_test_function_shape(
    file_path: &path::Path,
    item_fn: &syn::ItemFn,
) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    let name = item_fn.sig.ident.to_string();
    if !(name.starts_with("given_") && name.contains("_when_") && name.contains("_then_")) {
        violations.push(models::Violation::new(
            "SQT010",
            file_path,
            Some(item_fn.sig.ident.span().start().line),
            "test names must follow given_<state>_when_<action>_then_<expectation>",
        ));
    }

    let mut visitor = TestFunctionVisitor::default();
    visitor.visit_block(&item_fn.block);

    if visitor.has_if_or_match {
        violations.push(models::Violation::new(
            "SQT011",
            file_path,
            Some(item_fn.sig.ident.span().start().line),
            "test functions must not contain if or match; split scenarios into cases or helpers",
        ));
    }
    if visitor.has_unwrap {
        violations.push(models::Violation::new(
            "SQT012",
            file_path,
            Some(item_fn.sig.ident.span().start().line),
            "test functions must use expect with context instead of unwrap",
        ));
    }
    if visitor.assert_count == 0 {
        violations.push(models::Violation::new(
            "SQT013",
            file_path,
            Some(item_fn.sig.ident.span().start().line),
            "test functions must contain assertion macros",
        ));
    }
    if visitor.assert_without_expected {
        violations.push(models::Violation::new(
            "SQT014",
            file_path,
            Some(item_fn.sig.ident.span().start().line),
            "every assertion macro must reference test_case.expected_...",
        ));
    }

    violations
}

fn check_test_helpers_file(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    for item in &file.items {
        if let syn::Item::Fn(item_fn) = item {
            if is_test_fn(item_fn) {
                violations.push(models::Violation::new(
                    "SQT015",
                    file_path,
                    Some(item_fn.sig.ident.span().start().line),
                    "helpers.rs must not define test functions",
                ));
            }
        }
        if let syn::Item::Use(item_use) = item {
            if contains_wildcard(&item_use.tree) {
                violations.push(models::Violation::new(
                    "SQT006",
                    file_path,
                    Some(item_use.use_token.span.start().line),
                    "test files must not use wildcard imports",
                ));
            }
        }
    }
    violations
}

fn check_test_types_file(file_path: &path::Path, file: &syn::File) -> Vec<models::Violation> {
    let mut violations = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Use(_) | syn::Item::Type(_) => {}
            syn::Item::Struct(item_struct) => {
                if !item_struct.ident.to_string().ends_with("TestCase") {
                    continue;
                }
                let field_names = named_field_names(item_struct);
                if !field_names
                    .iter()
                    .any(|field_name| field_name == "description")
                {
                    violations.push(models::Violation::new(
                        "SQT016",
                        file_path,
                        Some(item_struct.ident.span().start().line),
                        "test case structs in test_types.rs must define a description field",
                    ));
                }
                if !field_names
                    .iter()
                    .any(|field_name| field_name.starts_with("expected_"))
                {
                    violations.push(models::Violation::new(
                        "SQT017",
                        file_path,
                        Some(item_struct.ident.span().start().line),
                        "test case structs in test_types.rs must define at least one expected_ field",
                    ));
                }
            }
            syn::Item::Impl(_) => {}
            _ => violations.push(models::Violation::new(
                "SQT018",
                file_path,
                item_line(item),
                "test_types.rs may contain only imports, type aliases, structs, and impls",
            )),
        }
    }
    violations
}

fn use_tree_paths(tree: &syn::UseTree) -> Vec<Vec<String>> {
    fn walk(tree: &syn::UseTree, mut prefix: Vec<String>, paths: &mut Vec<Vec<String>>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                walk(&path.tree, prefix, paths);
            }
            syn::UseTree::Name(name) => {
                prefix.push(name.ident.to_string());
                paths.push(prefix);
            }
            syn::UseTree::Rename(rename) => {
                prefix.push(rename.ident.to_string());
                paths.push(prefix);
            }
            syn::UseTree::Glob(_) => {
                prefix.push("*".to_owned());
                paths.push(prefix);
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    walk(item, prefix.clone(), paths);
                }
            }
        }
    }

    let mut paths = Vec::new();
    walk(tree, Vec::new(), &mut paths);
    paths
}

fn contains_wildcard(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Path(path) => contains_wildcard(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().any(contains_wildcard),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => false,
    }
}

#[derive(Default)]
struct TestFunctionVisitor {
    assert_count: usize,
    assert_without_expected: bool,
    has_if_or_match: bool,
    has_unwrap: bool,
}

impl<'ast> visit::Visit<'ast> for TestFunctionVisitor {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.has_if_or_match = true;
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.has_if_or_match = true;
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "unwrap" {
            self.has_unwrap = true;
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let macro_name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if matches!(
            macro_name.as_deref(),
            Some("assert" | "assert_eq" | "assert_ne")
        ) {
            self.assert_count += 1;
            let tokens = node.tokens.to_string();
            if !(tokens.contains("test_case . expected_") || tokens.contains("test_case.expected_"))
            {
                self.assert_without_expected = true;
            }
        }
        visit::visit_macro(self, node);
    }
}

fn is_test_fn(item_fn: &syn::ItemFn) -> bool {
    item_fn
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("test"))
}

fn named_field_names(item_struct: &syn::ItemStruct) -> Vec<String> {
    match &item_struct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .filter_map(|field| field.ident.as_ref().map(ToString::to_string))
            .collect(),
        syn::Fields::Unnamed(_) | syn::Fields::Unit => Vec::new(),
    }
}

fn is_public_item(item: &syn::Item) -> bool {
    match item {
        syn::Item::Const(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Enum(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Fn(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Mod(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Struct(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Trait(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Type(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Use(item) => matches!(item.vis, syn::Visibility::Public(_)),
        _ => false,
    }
}

fn item_line(item: &syn::Item) -> Option<usize> {
    Some(match item {
        syn::Item::Const(item) => item.const_token.span.start().line,
        syn::Item::Enum(item) => item.enum_token.span.start().line,
        syn::Item::Fn(item) => item.sig.ident.span().start().line,
        syn::Item::Impl(item) => item.impl_token.span.start().line,
        syn::Item::Mod(item) => item.mod_token.span.start().line,
        syn::Item::Static(item) => item.static_token.span.start().line,
        syn::Item::Struct(item) => item.struct_token.span.start().line,
        syn::Item::Trait(item) => item.trait_token.span.start().line,
        syn::Item::Type(item) => item.type_token.span.start().line,
        syn::Item::Use(item) => item.use_token.span.start().line,
        _ => return None,
    })
}

fn top_sqlquack_module(repo_root: &path::Path, file_path: &path::Path) -> Option<String> {
    let parts = relative_parts(repo_root, file_path);
    if parts.len() < 4 || parts[..3] != ["crates", "sqlquack", "src"] {
        return None;
    }
    if parts[3] == "lib.rs" {
        return None;
    }
    Some(parts[3].strip_suffix(".rs").unwrap_or(&parts[3]).to_owned())
}

fn relative_parts(repo_root: &path::Path, file_path: &path::Path) -> Vec<String> {
    file_path
        .strip_prefix(repo_root)
        .unwrap_or(file_path)
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(ToOwned::to_owned))
        .collect()
}

fn relative_path(repo_root: &path::Path, file_path: &path::Path) -> Option<String> {
    file_path
        .strip_prefix(repo_root)
        .ok()
        .and_then(|path| path.to_str())
        .map(|path| path.replace('\\', "/"))
}
