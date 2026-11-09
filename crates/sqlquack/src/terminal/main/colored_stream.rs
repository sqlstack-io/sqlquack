//! `colored_stream`: a standard stream as `--color` colours it, with the palette for its text.

use std::io::Write;

use anstream::stream::{AsLockedWrite, RawStream};
use clap::ColorChoice;
use sqlquack_diagnostics::diagnostics::constants::{ANSI_PALETTE, PLAIN_PALETTE};
use sqlquack_diagnostics::diagnostics::models::Palette;

use crate::terminal::_helpers::stream_color::stream_color;

/// `stream` as `choice` colours it, and the palette for what is written on it.
pub fn colored_stream<'a, S: RawStream + AsLockedWrite + 'a>(
    stream: S,
    choice: ColorChoice,
) -> (Box<dyn Write + 'a>, &'static Palette) {
    match stream_color(choice, &stream) {
        anstream::ColorChoice::Never => (Box::new(stream), &PLAIN_PALETTE),
        color => (
            Box::new(anstream::AutoStream::new(stream, color)),
            &ANSI_PALETTE,
        ),
    }
}
