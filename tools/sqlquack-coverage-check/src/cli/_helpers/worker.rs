//! Local parses in a child process, so a crash, hang or memory overrun fails one case only.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use crate::check::models::{LocalFailure, LocalVerdict};

/// The exit code of a Rust process whose main thread panicked.
const PANIC_EXIT: i32 = 101;

/// One live worker: its process, its input, and its output lines.
struct Running {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
}

/// Sends each SQL text, as a JSON string line, to a worker that prints one verdict line each.
pub(crate) struct Worker {
    command: Box<dyn Fn() -> Command>,
    bound: Duration,
    running: Option<Running>,
}

fn failed(failure: LocalFailure) -> LocalVerdict {
    LocalVerdict::Failed { failure }
}

fn start(command: &dyn Fn() -> Command) -> Option<Running> {
    let spawned = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = spawned else {
        return None;
    };
    let input: ChildStdin = child.stdin.take()?;
    let output = BufReader::new(child.stdout.take()?);
    let (sender, lines) = channel();
    std::thread::spawn(move || {
        for line in output.lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    Some(Running {
        child,
        input,
        lines,
    })
}

/// Stops a worker that may still run, and says why it died when it had exited on its own.
fn stop(mut running: Running) -> LocalFailure {
    let _ = running.child.kill();
    match running.child.wait() {
        Ok(status) if status.code() == Some(PANIC_EXIT) => LocalFailure::Panic,
        _ => LocalFailure::Crash,
    }
}

impl Worker {
    /// A worker started by `command`, restarted after every failure; each parse gets `bound`.
    pub(crate) fn new(command: Box<dyn Fn() -> Command>, bound: Duration) -> Self {
        Self {
            command,
            bound,
            running: None,
        }
    }

    /// The worker's verdict on `sql`, or how the worker failed to give one.
    pub(crate) fn verdict(&mut self, sql: &str) -> LocalVerdict {
        let Some(mut running) = self.running.take().or_else(|| start(&self.command)) else {
            return failed(LocalFailure::Crash);
        };
        let request: String = format!("{}\n", serde_json::Value::from(sql));
        if running.input.write_all(request.as_bytes()).is_err() || running.input.flush().is_err() {
            return failed(stop(running));
        }
        match running.lines.recv_timeout(self.bound) {
            Ok(line) => match serde_json::from_str::<LocalVerdict>(&line) {
                Ok(verdict) => {
                    self.running = Some(running);
                    verdict
                }
                Err(_) => failed(stop(running)),
            },
            Err(RecvTimeoutError::Timeout) => {
                stop(running);
                failed(LocalFailure::Timeout)
            }
            Err(RecvTimeoutError::Disconnected) => failed(stop(running)),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            stop(running);
        }
    }
}
