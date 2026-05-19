fn main() {
    let repo_root = std::env::current_dir().expect("current directory should be readable");
    let violations = sqlquack_structure_checker::rules::check_repo(&repo_root);

    for violation in &violations {
        let path = violation
            .path
            .strip_prefix(&repo_root)
            .unwrap_or(&violation.path);
        match violation.line {
            Some(line) => println!(
                "{}:{}: {}: {}",
                path.display(),
                line,
                violation.code,
                violation.message
            ),
            None => println!(
                "{}: {}: {}",
                path.display(),
                violation.code,
                violation.message
            ),
        }
    }

    if !violations.is_empty() {
        std::process::exit(1);
    }
}
