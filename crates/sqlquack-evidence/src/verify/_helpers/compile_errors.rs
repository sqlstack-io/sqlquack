//! Recorded compile-class errors, which a case must declare so the recorder's pre-check lets it run.

use crate::cases::models::LoadedCase;
use crate::constants::COMPILE_ERROR_SQLSTATE_CLASSES;
use crate::results::models::RecordedFailure;

/// Why `recorded` fails `case`: a compile-class error the case neither declares nor gets from setup.
pub(crate) fn undeclared_compile_error(
    case: &LoadedCase,
    recorded: Option<&RecordedFailure>,
) -> Option<String> {
    let sqlstate = recorded?.sqlstate.as_deref()?;
    (compile_class(sqlstate) && !case.compile_error && case.setup_sql.is_empty() && case.setup.is_empty()).then(|| {
        format!(
            "the recording is a compile-class Snowflake error (SQLSTATE {sqlstate}), which the case \
             must declare with compile_error = true"
        )
    })
}

/// Whether `recorded` shows Snowflake refusing to compile `case`'s statement, as the case declares.
pub(crate) fn rejected_at_compile_time(
    case: &LoadedCase,
    recorded: Option<&RecordedFailure>,
) -> bool {
    case.compile_error
        && recorded
            .and_then(|recorded| recorded.sqlstate.as_deref())
            .is_some_and(compile_class)
}

fn compile_class(sqlstate: &str) -> bool {
    COMPILE_ERROR_SQLSTATE_CLASSES
        .iter()
        .any(|class| sqlstate.starts_with(class))
}
