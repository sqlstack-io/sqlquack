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
    let compile_class = COMPILE_ERROR_SQLSTATE_CLASSES
        .iter()
        .any(|class| sqlstate.starts_with(class));
    (compile_class && !case.compile_error && case.setup_sql.is_empty()).then(|| {
        format!(
            "the recording is a compile-class Snowflake error (SQLSTATE {sqlstate}), which the case \
             must declare with compile_error = true"
        )
    })
}
