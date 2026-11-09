//! `render_styled_diagnostics`: diagnostics as rustc-style text in a palette, spans underlined.

use std::fmt::Write;

use crate::diagnostics::_helpers::line_index::LineIndex;
use crate::diagnostics::constants::{MAX_SHOWN_LINE, TAB_WIDTH};
use crate::diagnostics::models::{Diagnostic, DiagnosticSpan, Palette};

/// Every diagnostic of `source` at `path`, styled by `style`; the plain palette is the plain text.
pub fn render_styled_diagnostics(
    diagnostics: &[Diagnostic],
    source: &str,
    path: &str,
    style: &Palette,
) -> String {
    let lines = LineIndex::new(source);
    diagnostics
        .iter()
        .map(|diagnostic| render_one(diagnostic, &lines, path, style))
        .collect::<Vec<String>>()
        .join("\n")
}

fn render_one(
    diagnostic: &Diagnostic,
    lines: &LineIndex<'_>,
    path: &str,
    style: &Palette,
) -> String {
    let Palette {
        message,
        gutter: blue,
        note,
        help,
        reset,
        ..
    } = *style;
    let severity = style.severity(diagnostic.severity);
    let mut text = format!(
        "{severity}{}[{}]{reset}{message}: {}{reset}\n",
        diagnostic.severity.label(),
        diagnostic.code,
        diagnostic.message
    );
    let gutter = " ".repeat(
        diagnostic
            .span
            .map_or(1, |span| span.line.to_string().len()),
    );
    match diagnostic.span {
        Some(span) => {
            let _ = writeln!(
                text,
                "{gutter}{blue}-->{reset} {path}:{}:{}",
                span.line, span.column
            );
            let (shown, underline) = excerpt(lines.text(span.line), span);
            let _ = writeln!(text, "{gutter} {blue}|{reset}");
            let _ = writeln!(text, "{blue}{} |{reset} {shown}", span.line);
            let _ = writeln!(
                text,
                "{gutter} {blue}|{reset} {severity}{underline} {}{reset}",
                diagnostic.label
            );
            if !diagnostic.notes.is_empty() || diagnostic.help.is_some() {
                let _ = writeln!(text, "{gutter} {blue}|{reset}");
            }
        }
        None => {
            let _ = writeln!(text, "{gutter}{blue}-->{reset} {path}");
            let _ = writeln!(text, "{gutter} {blue}|{reset}");
            let _ = writeln!(
                text,
                "{gutter} {blue}={reset} {note}note{reset}: {}",
                diagnostic.label
            );
        }
    }
    for line in &diagnostic.notes {
        let _ = writeln!(text, "{gutter} {blue}={reset} {note}note{reset}: {line}");
    }
    if let Some(advice) = &diagnostic.help {
        let _ = writeln!(text, "{gutter} {blue}={reset} {help}help{reset}: {advice}");
    }
    text
}

/// The line as shown, windowed when long and with tabs expanded, and its underline.
fn excerpt(line: &str, span: DiagnosticSpan) -> (String, String) {
    let characters: Vec<char> = line.chars().collect();
    let first = span.column.saturating_sub(1).min(characters.len());
    let last = match span.end_line == span.line {
        true => span
            .end_column
            .saturating_sub(1)
            .clamp(first, characters.len()),
        false => characters.len(),
    }
    .max(first + 1);
    let window_start = match characters.len() > MAX_SHOWN_LINE {
        true => first.saturating_sub(MAX_SHOWN_LINE / 2),
        false => 0,
    };
    let window_end = (window_start + MAX_SHOWN_LINE).min(characters.len());
    let mut shown = String::new();
    let mut underline = String::new();
    if window_start > 0 {
        shown.push('…');
        underline.push(' ');
    }
    for (index, character) in characters[window_start..window_end].iter().enumerate() {
        let offset = window_start + index;
        let width = match character {
            '\t' => TAB_WIDTH,
            _ => 1,
        };
        match character {
            '\t' => shown.push_str(&" ".repeat(TAB_WIDTH)),
            other => shown.push(*other),
        }
        let mark = match (first..last).contains(&offset) {
            true => '^',
            false => ' ',
        };
        underline.push_str(&mark.to_string().repeat(width));
    }
    if window_end < characters.len() {
        shown.push('…');
    }
    let marked = last.min(window_end.max(first + 1));
    for _ in window_end.max(first)..marked {
        underline.push('^');
    }
    (shown, underline.trim_end().to_owned())
}
