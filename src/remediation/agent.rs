use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{self, Command, Stdio};
use std::time::Duration;

pub fn invoke_agent(
    command: &str,
    payload_json: &str,
    timeout_secs: u64,
    env: &HashMap<String, String>,
) -> Result<String> {
    let parts = shell_words::split(command)
        .with_context(|| format!("failed to parse agent_command: {}", command))?;
    let program = parts.first().context("agent_command must not be empty")?;
    let args = &parts[1..];

    let mut child = Command::new(program)
        .args(args)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn agent process: {}", command))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload_json.as_bytes())
            .context("failed to write payload to agent stdin")?;
    }

    // Read stdout on a background thread to avoid pipe deadlock.
    let stdout_handle = child.stdout.take();
    let reader = std::thread::spawn(move || -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        if let Some(mut handle) = stdout_handle {
            handle
                .read_to_end(&mut buf)
                .context("failed to read agent stdout")?;
        }
        Ok(buf)
    });

    // Save PID before moving child into the wait thread
    let child_pid = child.id();
    let timeout = Duration::from_secs(timeout_secs);
    let cmd_str = command.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });

    let status = match rx.recv_timeout(timeout) {
        Ok(result) => result.context("failed to wait on agent process")?,
        Err(_) => {
            // kill(2) via the nix-free route: just send SIGKILL by PID
            #[cfg(unix)]
            {
                let _ = process::Command::new("kill")
                    .args(["-9", &child_pid.to_string()])
                    .status();
            }
            bail!(
                "agent process timed out after {}s: {}",
                timeout_secs,
                cmd_str
            );
        }
    };

    let stdout_bytes = reader
        .join()
        .map_err(|_| anyhow::anyhow!("stdout reader thread panicked"))?
        .context("failed to read agent stdout")?;

    if !status.success() {
        bail!("agent process exited with status {}: {}", status, command);
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
    Ok(stdout)
}
