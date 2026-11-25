//! Spawning ffmpeg/ffprobe: argv only (no shell), stderr always drained.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;

use crate::MediaError;

/// Bytes of stderr kept for error messages.
const TAIL: usize = 4096;

/// Binary from env var `env`, else `default` looked up on PATH.
pub(crate) fn bin(env: &str, default: &str) -> OsString {
    std::env::var_os(env)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.into())
}

/// `file:` keeps ffmpeg from reading a path as an option (`-x.mp4`) or a protocol (`a:b`).
pub(crate) fn input(path: &str) -> String {
    format!("file:{path}")
}

pub(crate) fn command(program: &OsStr) -> Command {
    let mut c = Command::new(program);
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: a GUI host would otherwise flash a console per ffmpeg launch.
        c.creation_flags(0x0800_0000);
    }
    c
}

pub(crate) fn spawn_err(program: &OsStr, e: std::io::Error) -> MediaError {
    if e.kind() == std::io::ErrorKind::NotFound {
        MediaError::FfmpegMissing(format!("{}: {e}", program.to_string_lossy()))
    } else {
        MediaError::Io(format!("starting {}: {e}", program.to_string_lossy()))
    }
}

fn tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(TAIL);
    String::from_utf8_lossy(&bytes[start..]).trim().to_string()
}

fn failed(status: std::process::ExitStatus, stderr: &[u8]) -> MediaError {
    let t = tail(stderr);
    MediaError::Ffmpeg(if t.is_empty() {
        format!("exited with {status}")
    } else {
        t
    })
}

/// Runs to completion and returns (stdout, stderr). `Command::output` reads both pipes
/// concurrently, so neither can fill up and deadlock.
pub(crate) fn run(mut cmd: Command) -> Result<(Vec<u8>, String), MediaError> {
    let out = cmd.output().map_err(|e| spawn_err(cmd.get_program(), e))?;
    if !out.status.success() {
        return Err(failed(out.status, &out.stderr));
    }
    Ok((
        out.stdout,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// A long-running child whose stderr a thread drains (keeping the tail). Killed on drop.
pub(crate) struct Proc {
    pub child: Child,
    stderr: Option<JoinHandle<Vec<u8>>>,
}

impl Proc {
    pub fn spawn(mut cmd: Command) -> Result<Self, MediaError> {
        let mut child = cmd.spawn().map_err(|e| spawn_err(cmd.get_program(), e))?;
        let mut pipe = child.stderr.take().expect("stderr is piped");
        let stderr = std::thread::spawn(move || {
            let mut keep = Vec::new();
            let mut buf = [0u8; 4096];
            while let Ok(k @ 1..) = pipe.read(&mut buf) {
                keep.extend_from_slice(&buf[..k]);
                if keep.len() > 2 * TAIL {
                    keep.drain(..keep.len() - TAIL);
                }
            }
            keep
        });
        Ok(Proc {
            child,
            stderr: Some(stderr),
        })
    }

    /// Waits for exit; `Err` carrying the stderr tail if it failed.
    pub fn wait(&mut self) -> Result<(), MediaError> {
        let status = self
            .child
            .wait()
            .map_err(|e| MediaError::Io(format!("waiting for ffmpeg: {e}")))?;
        let err = self
            .stderr
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        if status.success() {
            Ok(())
        } else {
            Err(failed(status, &err))
        }
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
