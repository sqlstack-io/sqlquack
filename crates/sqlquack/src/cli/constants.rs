//! Command-line text shared by every command.

/// What each exit status means, shown after the top-level help.
pub const EXIT_STATUS_HELP: &str = "Exit status of check, convert and function:\n  \
    0  the command succeeded\n  \
    1  check found a problem, or something was refused; the reasons are on standard error\n  \
    2  the command could not run: bad arguments, an unreadable file, or input without SQL\n\
    census and index exit 0 when they finish, refused files included, and 2 when they cannot run.";
