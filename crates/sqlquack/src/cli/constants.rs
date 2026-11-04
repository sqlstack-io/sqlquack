//! Command-line text shared by every command.

/// What each exit status means, shown after the top-level help.
pub const EXIT_STATUS_HELP: &str = "Exit status of convert, function, census and index:\n  0  the command succeeded\n  1  something was refused; the reasons are on standard error\n  2  the command could not run: bad arguments, an unreadable file, or input without SQL";
