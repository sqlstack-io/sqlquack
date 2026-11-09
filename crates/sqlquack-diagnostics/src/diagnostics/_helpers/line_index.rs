//! Where each source line starts, in characters and bytes, built once per source.

use crate::diagnostics::models::DiagnosticSpan;
use sqlquack_syntax::tokens::models::SourceLocation;

pub(crate) struct LineIndex<'source> {
    source: &'source str,
    /// Character and byte offset of each line's first character.
    starts: Vec<(usize, usize)>,
    length: usize,
}

impl<'source> LineIndex<'source> {
    pub(crate) fn new(source: &'source str) -> Self {
        let mut starts = vec![(0, 0)];
        let mut length = 0;
        for (index, (byte, character)) in source.char_indices().enumerate() {
            if character == '\n' {
                starts.push((index + 1, byte + 1));
            }
            length = index + 1;
        }
        Self {
            source,
            starts,
            length,
        }
    }

    /// The zero-based line holding character `offset`.
    fn line_of(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|(start, _)| *start <= offset)
            .saturating_sub(1)
    }

    /// A refusal's location with the line and column its span ends on.
    pub(crate) fn span(&self, location: SourceLocation) -> DiagnosticSpan {
        let start = location.start.min(self.length);
        let end = location.end.clamp(start, self.length);
        let last = match end > start {
            true => end - 1,
            false => start,
        };
        let end_line = self.line_of(last);
        let end_column = match end > start {
            true => end - self.starts[end_line].0 + 1,
            false => start - self.starts[end_line].0 + 1,
        };
        let line = self.line_of(start);
        DiagnosticSpan {
            start,
            end,
            line: line + 1,
            column: start - self.starts[line].0 + 1,
            end_line: end_line + 1,
            end_column,
        }
    }

    /// The text of one-based `line`, without its line break; empty past the last line.
    pub(crate) fn text(&self, line: usize) -> &'source str {
        let Some(&(_, from)) = self.starts.get(line.saturating_sub(1)) else {
            return "";
        };
        let to = self
            .starts
            .get(line)
            .map_or(self.source.len(), |(_, byte)| *byte);
        self.source
            .get(from..to)
            .unwrap_or_default()
            .trim_end_matches(['\n', '\r'])
    }
}
