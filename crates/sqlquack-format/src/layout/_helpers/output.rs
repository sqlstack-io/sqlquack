//! Writing words and line breaks to the formatted text.

use crate::layout::_helpers::words::spaced;
use crate::layout::constants::INDENT_WIDTH;
use crate::layout::models::Printer;

impl Printer<'_, '_> {
    /// Start a new line at indent `level`.
    pub(crate) fn newline(&mut self, level: usize) {
        let output = &mut self.output;
        output.level = level;
        output.line_start = true;
        if output.text.is_empty() {
            return;
        }
        let kept = output.text.trim_end_matches(' ').len();
        output.text.truncate(kept);
        output.text.push('\n');
        output.text.push_str(&" ".repeat(level * INDENT_WIDTH));
        output.column = level * INDENT_WIDTH;
        output.after_line_comment = false;
    }

    /// Leave one empty line, then start a line at indent `level`.
    pub(crate) fn blank_line(&mut self, level: usize) {
        self.newline(0);
        self.newline(level);
    }

    /// Write the word at `index` on the current line, after a space where it takes one.
    pub(crate) fn word(&mut self, index: usize) {
        if self.output.after_line_comment {
            self.newline(self.output.level);
        }
        let space = spaced(self.words, index);
        let word = &self.words.words[index];
        let output = &mut self.output;
        if !output.line_start && !output.text.is_empty() && space {
            output.text.push(' ');
            output.column += 1;
        }
        output.text.push_str(word.text);
        output.column += word.text.chars().count();
        output.line_start = false;
        output.after_line_comment = word.line_comment;
    }

    /// Write the words `from..to` on the current line, as they were spaced.
    pub(crate) fn flat(&mut self, from: usize, to: usize) {
        (from..to).for_each(|index| self.word(index));
    }
}
