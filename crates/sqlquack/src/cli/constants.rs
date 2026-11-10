//! Command-line text shared by every command.

/// What each exit status means, shown after the top-level help.
pub const EXIT_STATUS_HELP: &str = "Exit status of check, convert, function and run:\n  \
    0  the command succeeded\n  \
    1  check found a problem, something was refused, or run's rows differ from --expect\n  \
    2  the command could not run: bad arguments, an unreadable file, or input without SQL\n\
    census exits 0 when every file converts, 1 when any is blocked, and 2 when it cannot run.\n\
    index exits 0 when it finishes, refused files included, and 2 when it cannot run.";
