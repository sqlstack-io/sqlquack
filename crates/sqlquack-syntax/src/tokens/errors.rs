//! Errors of tokens.

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::tokens::models::ColumnResolutionReason;
use crate::tokens::models::ColumnResolutionTarget;
use crate::tokens::models::Span;
use crate::tokens::models::ValidationSeverity;

/// Errors that can occur during SQL parsing and generation
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Error during tokenization
    #[error("Tokenization error at line {line}, column {column}: {message}")]
    Tokenize {
        message: String,
        line: usize,
        column: usize,
        start: usize,
        end: usize,
    },

    /// Error during parsing
    #[error("Parse error at line {line}, column {column}: {message}")]
    Parse {
        message: String,
        line: usize,
        column: usize,
        start: usize,
        end: usize,
    },

    /// Error during SQL generation
    #[error("Generation error: {0}")]
    Generate(String),

    /// Unsupported feature for the target dialect
    #[error("Unsupported: {feature} is not supported in {dialect}")]
    Unsupported { feature: String, dialect: String },

    /// Invalid SQL syntax
    #[error("Syntax error at line {line}, column {column}: {message}")]
    Syntax {
        message: String,
        line: usize,
        column: usize,
        start: usize,
        end: usize,
    },

    /// Invalid input for an operation after parsing has completed.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// A requested output column could not be resolved.
    #[error("Cannot resolve {target}: {reason}")]
    ColumnResolution {
        target: ColumnResolutionTarget,
        reason: ColumnResolutionReason,
    },

    /// Internal error (should not happen in normal usage)
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a tokenization error at `span`.
    pub fn tokenize(message: impl Into<String>, span: Span) -> Self {
        Error::Tokenize {
            message: message.into(),
            line: span.line,
            column: span.column,
            start: span.start,
            end: span.end,
        }
    }

    /// Create a parse error at `span`.
    pub fn parse(message: impl Into<String>, span: Span) -> Self {
        Error::Parse {
            message: message.into(),
            line: span.line,
            column: span.column,
            start: span.start,
            end: span.end,
        }
    }

    /// Get the line number if available
    pub fn line(&self) -> Option<usize> {
        match self {
            Error::Tokenize { line, .. }
            | Error::Parse { line, .. }
            | Error::Syntax { line, .. } => Some(*line),
            _ => None,
        }
    }

    /// Get the column number if available
    pub fn column(&self) -> Option<usize> {
        match self {
            Error::Tokenize { column, .. }
            | Error::Parse { column, .. }
            | Error::Syntax { column, .. } => Some(*column),
            _ => None,
        }
    }

    /// Get the start byte offset if available
    pub fn start(&self) -> Option<usize> {
        match self {
            Error::Tokenize { start, .. }
            | Error::Parse { start, .. }
            | Error::Syntax { start, .. } => Some(*start),
            _ => None,
        }
    }

    /// Get the end byte offset if available
    pub fn end(&self) -> Option<usize> {
        match self {
            Error::Tokenize { end, .. } | Error::Parse { end, .. } | Error::Syntax { end, .. } => {
                Some(*end)
            }
            _ => None,
        }
    }

    /// Create a generation error
    pub fn generate(message: impl Into<String>) -> Self {
        Error::Generate(message.into())
    }

    /// Create an unsupported feature error
    pub fn unsupported(feature: impl Into<String>, dialect: impl Into<String>) -> Self {
        Error::Unsupported {
            feature: feature.into(),
            dialect: dialect.into(),
        }
    }

    /// Create a syntax error at `span`.
    pub fn syntax(message: impl Into<String>, span: Span) -> Self {
        Error::Syntax {
            message: message.into(),
            line: span.line,
            column: span.column,
            start: span.start,
            end: span.end,
        }
    }

    /// Create an invalid-input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Error::InvalidInput(message.into())
    }

    /// Create a structured column-resolution error.
    pub fn column_resolution(
        target: ColumnResolutionTarget,
        reason: ColumnResolutionReason,
    ) -> Self {
        Error::ColumnResolution { target, reason }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Error::Internal(message.into())
    }
}

/// A single validation error or warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// The error/warning message
    pub message: String,
    /// Line number where the error occurred (1-based)
    pub line: Option<usize>,
    /// Column number where the error occurred (1-based)
    pub column: Option<usize>,
    /// Severity of the validation issue
    pub severity: ValidationSeverity,
    /// Error code (e.g., "E001", "W001")
    pub code: String,
    /// Start Unicode character offset of the error range (not a UTF-8 byte offset)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    /// End Unicode character offset of the error range (exclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn error(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
            severity: ValidationSeverity::Error,
            code: code.into(),
            start: None,
            end: None,
        }
    }

    /// Create a new validation warning
    pub fn warning(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
            severity: ValidationSeverity::Warning,
            code: code.into(),
            start: None,
            end: None,
        }
    }

    /// Set the line number
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set both line and column
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Set the start/end byte offsets
    pub fn with_span(mut self, start: Option<usize>, end: Option<usize>) -> Self {
        self.start = start;
        self.end = end;
        self
    }
}
