//! `census_with_progress`: the census, reporting progress on stderr while files convert.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::census::_helpers::timed::run_census_timed;
use crate::census::constants::PROGRESS_INTERVAL_SECONDS;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusProgress, CensusRun};

/// The census, handing `report` a progress line every few seconds while files convert.
pub fn census_with_progress(
    paths: &[PathBuf],
    options: &CensusOptions,
    report: &mut dyn FnMut(&str),
) -> Result<CensusRun, CensusError> {
    let progress = CensusProgress::default();
    let started = Instant::now();
    let (finished, waiting) = mpsc::channel();
    std::thread::scope(|scope| {
        let progress = &progress;
        let worker = scope.spawn(move || {
            let run = run_census_timed(paths, options, progress);
            let _ = finished.send(());
            run
        });
        while let Err(RecvTimeoutError::Timeout) =
            waiting.recv_timeout(Duration::from_secs(PROGRESS_INTERVAL_SECONDS))
        {
            let done = progress.done.load(Ordering::Relaxed);
            report(&format!(
                "sqlquack census: pass {}, {done} of {} files, {:.0} files/s overall",
                progress.pass.load(Ordering::Relaxed),
                progress.total.load(Ordering::Relaxed),
                progress.inspected.load(Ordering::Relaxed) as f64
                    / started.elapsed().as_secs_f64().max(f64::EPSILON),
            ));
        }
        worker
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}
