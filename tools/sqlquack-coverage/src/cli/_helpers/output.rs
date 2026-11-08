//! Printing: JSON alone on stdout, or short text lines; progress on stderr.

use std::io::Write;

use serde::Serialize;

pub(crate) fn print<T: Serialize>(json: bool, value: &T, text: &[String]) {
    let mut stdout = std::io::stdout().lock();
    if json {
        let _ = writeln!(
            stdout,
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    } else {
        for line in text {
            let _ = writeln!(stdout, "{line}");
        }
    }
}

/// One progress or problem line on stderr.
pub(crate) fn say(line: &str) {
    let _ = writeln!(std::io::stderr().lock(), "syntax-coverage: {line}");
}
