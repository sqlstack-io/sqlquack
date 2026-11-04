//! One case's conversion in its own `sqlquack convert` process, bounded in time and memory.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

use crate::translation::constants::CONVERSION_MEMORY_KIB;
use crate::translation::models::ConvertOutput;

/// How to run `sqlquack convert`: the binary, the policy, and the time each case may take.
pub(crate) struct Converter<'a> {
    pub(crate) binary: &'a Path,
    pub(crate) policy: &'a Path,
    pub(crate) bound: Duration,
}

impl Converter<'_> {
    /// Conversion's report on `sql`, with `schema` when given, or why it gave none.
    pub(crate) fn convert(
        &self,
        sql: &str,
        schema: Option<&Path>,
    ) -> Result<ConvertOutput, String> {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(format!(
                "ulimit -v {CONVERSION_MEMORY_KIB} && exec \"$0\" \"$@\""
            ))
            .arg(self.binary)
            .args(["convert", "--loud-divergences-only", "--json", "--policy"])
            .arg(self.policy);
        if let Some(schema) = schema {
            command.arg("--schema").arg(schema);
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|error| format!("cannot start sqlquack convert: {error}"))?;
        let mut input = child.stdin.take().ok_or("no stdin")?;
        let mut output = child.stdout.take().ok_or("no stdout")?;
        let (sender, received) = channel();
        let sql: String = sql.to_owned();
        std::thread::spawn(move || {
            let written: bool = input.write_all(sql.as_bytes()).is_ok();
            drop(input);
            let mut text = String::new();
            let read: bool = output.read_to_string(&mut text).is_ok();
            let _ = sender.send((written && read).then_some(text));
        });
        let text: Option<String> = match received.recv_timeout(self.bound) {
            Ok(text) => text,
            Err(RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timeout after {} s", self.bound.as_secs()));
            }
            Err(RecvTimeoutError::Disconnected) => None,
        };
        let status = child
            .wait()
            .map_err(|error| format!("sqlquack convert: {error}"))?;
        match (status.code(), text) {
            (Some(0 | 1), Some(text)) => serde_json::from_str(&text)
                .map_err(|error| format!("unreadable convert report: {error}")),
            (code, _) => Err(format!(
                "sqlquack convert exited {}",
                code.map_or_else(|| "on a signal".to_owned(), |code| code.to_string())
            )),
        }
    }
}
