//! `window_frame`: a window frame clause, spelled out.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::WINDOW_FRAME;

/// A window frame clause, spelled out.
pub fn window_frame(frame: &str, log: &ProvenanceLog) -> String {
    log.building_block(WINDOW_FRAME);
    frame.to_owned()
}
