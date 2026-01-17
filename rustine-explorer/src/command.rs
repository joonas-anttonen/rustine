use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Result of executing or starting an external command.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Process was started (not waited for). Contains the child PID.
    Started(u32),
    /// Process completed; contains exit status, stdout and stderr.
    Completed {
        status: std::process::ExitStatus,
        stdout: String,
        stderr: String,
    },
}

/// Run an external `program` with `args` and append `path` as the last argument.
/// Waits for completion and returns captured stdout/stderr and exit status.
pub fn run_command_with_path(
    program: &str,
    args: &[&str],
    path: impl AsRef<Path>,
) -> io::Result<CommandResult> {
    let path = path.as_ref();

    let output = Command::new(program)
        .args(args)
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok(CommandResult::Completed {
        status: output.status,
        stdout,
        stderr,
    })
}

/// Spawn an external `program` with `args` and append `path` as the last argument.
/// Returns immediately with the child PID. The process keeps running in background.
pub fn spawn_command_with_path(
    program: &str,
    args: &[&str],
    path: impl AsRef<Path>,
) -> io::Result<CommandResult> {
    let path = path.as_ref();

    let mut cmd = Command::new(program);
    cmd.args(args).arg(path);

    // On most platforms spawn() is sufficient; return the pid so UI can show progress.
    let child = cmd.spawn()?;
    let pid = child.id();

    // We intentionally don't wait here; dropping `child` will not kill the process.
    Ok(CommandResult::Started(pid))
}
