use kedge::remediation::agent::invoke_agent;
use std::collections::HashMap;

#[test]
fn captures_stdout() {
    let result = invoke_agent("echo hello", "", 5, &HashMap::new());
    assert_eq!(result.unwrap().trim(), "hello");
}

#[test]
fn passes_payload_via_stdin() {
    let result = invoke_agent("cat", "{\"key\":\"value\"}", 5, &HashMap::new());
    assert_eq!(result.unwrap().trim(), "{\"key\":\"value\"}");
}

#[test]
fn passes_env_vars_to_child() {
    let mut env = HashMap::new();
    env.insert("STEER_TEST_VAR".to_string(), "42".to_string());
    let result = invoke_agent("sh -c 'echo $STEER_TEST_VAR'", "", 5, &env);
    assert_eq!(result.unwrap().trim(), "42");
}

#[test]
fn non_zero_exit_returns_error() {
    let result = invoke_agent("false", "", 5, &HashMap::new());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("exited with status"), "got: {}", err);
}

#[test]
fn timeout_kills_process_and_returns_error() {
    let result = invoke_agent("sleep 60", "", 2, &HashMap::new());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("timed out"), "got: {}", err);
}

#[test]
fn empty_command_returns_error() {
    let result = invoke_agent("", "", 5, &HashMap::new());
    assert!(result.is_err());
}

#[test]
fn handles_shell_quoting() {
    let result = invoke_agent("echo 'hello world'", "", 5, &HashMap::new());
    assert_eq!(result.unwrap().trim(), "hello world");
}

#[test]
fn returns_empty_string_for_no_output() {
    let result = invoke_agent("true", "", 5, &HashMap::new());
    assert_eq!(result.unwrap().trim(), "");
}

#[test]
fn handles_large_stdout() {
    // Generate ~100KB of output to exercise the background reader thread
    let result = invoke_agent("sh -c 'seq 1 10000'", "", 10, &HashMap::new());
    let output = result.unwrap();
    assert!(output.lines().count() >= 10000);
}

#[test]
fn handles_large_stdin_and_stdout() {
    // Pipe a large payload through cat — exercises both stdin write and stdout read
    let payload = "x".repeat(100_000);
    let result = invoke_agent("cat", &payload, 10, &HashMap::new());
    assert_eq!(result.unwrap().len(), 100_000);
}

#[test]
fn invalid_command_returns_spawn_error() {
    let result = invoke_agent("nonexistent_binary_xyz", "", 5, &HashMap::new());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("failed to spawn"), "got: {}", err);
}

#[test]
fn stderr_does_not_appear_in_stdout() {
    let result = invoke_agent("sh -c 'echo out; echo err >&2'", "", 5, &HashMap::new());
    let output = result.unwrap();
    assert_eq!(output.trim(), "out");
}
