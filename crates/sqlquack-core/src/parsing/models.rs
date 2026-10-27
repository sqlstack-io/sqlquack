//! Dialects and source positions of parsed SQL.

use serde::Serialize;
use sqlquack_syntax::Dialect;
/// A dialect SQLQuack converts from. Every entry point takes it as a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDialect {
    /// Snowflake SQL.
    Snowflake,
}

impl SourceDialect {
    /// The parser and printer dialect for this source.
    pub fn syntax(self) -> Dialect {
        match self {
            Self::Snowflake => Dialect::Snowflake,
        }
    }

    /// The lower-case name used on the command line and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Snowflake => "snowflake",
        }
    }

    /// The source dialect with this command-line name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "snowflake" => Some(Self::Snowflake),
            _ => None,
        }
    }
}

/// A position in source SQL, in Unicode characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct SourceLocation {
    /// Character offset of the first character.
    pub start: usize,
    /// Character offset one past the last character.
    pub end: usize,
    /// One-based line.
    pub line: usize,
    /// One-based column.
    pub column: usize,
}

/// Where each line of source SQL starts, to turn character ranges into locations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLines {
    line_starts: Vec<usize>,
    length: usize,
    shift: usize,
}

impl SourceLines {
    /// The lines of `source`.
    pub fn new(source: &str) -> Self {
        Self::shifted(source, 0)
    }

    /// The lines of `source`, for ranges in a parse of it behind a prefix of `shift` characters.
    pub fn shifted(source: &str, shift: usize) -> Self {
        let mut line_starts = vec![0];
        let mut length = 0;
        for (offset, character) in source.chars().enumerate() {
            if character == '\n' {
                line_starts.push(offset + 1);
            }
            length = offset + 1;
        }
        Self {
            line_starts,
            length,
            shift,
        }
    }

    /// The location of the characters `start..end` of the parsed text.
    pub fn location(&self, start: usize, end: usize) -> SourceLocation {
        let start = start.saturating_sub(self.shift).min(self.length);
        let end = end.saturating_sub(self.shift).clamp(start, self.length);
        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= start)
            .saturating_sub(1);
        SourceLocation {
            start,
            end,
            line: line_index + 1,
            column: start - self.line_starts[line_index] + 1,
        }
    }

    /// The location of the whole source.
    pub fn whole(&self) -> SourceLocation {
        self.location(self.shift, self.shift + self.length)
    }
}
