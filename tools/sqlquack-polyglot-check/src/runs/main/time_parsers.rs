//! `time_parsers`: each parser's median time over a whole corpus.

use std::time::Duration;

use crate::corpora::models::CorpusEntry;
use crate::runs::_helpers::passes::{median, polyglot_pass, sqlquack_pass};
use crate::runs::models::ParserTimes;

/// Median whole-corpus parse times over `rounds` rounds, alternating which parser runs first.
pub fn time_parsers(entries: &[CorpusEntry], rounds: usize) -> ParserTimes {
    let mut polyglot: Vec<Duration> = Vec::with_capacity(rounds);
    let mut sqlquack: Vec<Duration> = Vec::with_capacity(rounds);
    for round in 0..rounds {
        if round.is_multiple_of(2) {
            polyglot.push(polyglot_pass(entries));
            sqlquack.push(sqlquack_pass(entries));
        } else {
            sqlquack.push(sqlquack_pass(entries));
            polyglot.push(polyglot_pass(entries));
        }
    }
    ParserTimes {
        polyglot: median(polyglot),
        sqlquack: median(sqlquack),
    }
}
