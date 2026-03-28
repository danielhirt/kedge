use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Invoke an external agent process, passing `payload_json` to its stdin.
///
/// The command string is split on whitespace to derive program + arguments.
/// Stdout is captured and returned on success; stderr is inherited (printed to
/// the parent's stderr so the caller can see agent diagnostics).
pub fn invoke_agent(command: &str, payload_json: &str) -> Result<String> {
    let mut parts = command.split_whitespace();
    let program = parts
        .next()
        .context("agent_command must not be empty")?;
    let args: Vec<&str> = parts.collect();

    let mut child = Command::new(program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn agent process: {}", command))?;

    // Write the payload JSON to the child's stdin.
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(payload_json.as_bytes())
            .context("failed to write payload to agent stdin")?;
    }
    // Dropping stdin closes the pipe, signalling EOF to the child.

    let output = child
        .wait_with_output()
        .context("failed to wait for agent process")?;

    if !output.status.success() {
        bail!(
            "agent process exited with status {}: {}",
            output.status,
            command
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(stdout)
}
