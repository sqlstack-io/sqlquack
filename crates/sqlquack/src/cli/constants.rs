//! Command-line text shared by every command.

/// What each exit status means, shown after the top-level help.
pub const EXIT_STATUS_HELP: &str = "Exit status of check, convert, function and run:\n  \
    0  the command succeeded\n  \
    1  check found a problem, something was refused, or run's rows differ from --expect\n  \
    2  the command could not run: bad arguments, an unreadable file, or input without SQL\n\
    census and index exit 0 when they finish, refused files included, and 2 when they cannot run.";
