//! `stream_color`: whether `check` and `convert` colour a stream, as `--color` and the environment say.

use anstream::stream::RawStream;
use clap::ColorChoice;

/// How `stream` is coloured: `auto` colours a terminal unless NO_COLOR, or any stream when forced.
pub(crate) fn stream_color<S: RawStream>(choice: ColorChoice, stream: &S) -> anstream::ColorChoice {
    match choice {
        ColorChoice::Always => anstream::ColorChoice::Always,
        ColorChoice::Never => anstream::ColorChoice::Never,
        ColorChoice::Auto if set("NO_COLOR") => anstream::ColorChoice::Never,
        ColorChoice::Auto if set("FORCE_COLOR") => anstream::ColorChoice::Always,
        ColorChoice::Auto => anstream::AutoStream::choice(stream),
    }
}

/// Whether the environment variable `name` is set and not empty, as NO_COLOR and FORCE_COLOR mean.
fn set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}
