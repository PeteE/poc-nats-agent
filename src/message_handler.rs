use anyhow::{Result, Context, bail};
use tracing::{info, error};
use crate::event::Event;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use std::env;

/// Handle a message event by executing an external handler program
///
/// The handler program should:
/// - Read a JSON event from stdin (can be any JSON structure)
/// - Process the event
/// - Exit with code 0 on success, 1 on error
/// - Optionally write to stdout (success) or stderr (error)
///
/// The NATS subject is the single source of truth for what kind of event
/// this is. Since only the JSON body reaches the handler's stdin, the
/// subject is passed through the environment instead:
/// - `NATS_SUBJECT` - the subject the message was published to
/// - `NATS_MSG_ID`  - the event's `id` field, if it has one
pub async fn handle_message(event: &Event, subject: &str) -> Result<()> {
    // Extract id if present, for logging and to expose to the handler
    let event_id = event.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    info!("Handling message - ID: {}, Subject: {}", event_id, subject);

    // Get the handler command from environment variable
    let handler_cmd = env::var("MESSAGE_HANDLER_CMD")
        .context("MESSAGE_HANDLER_CMD environment variable not set")?;

    // Serialize the event to JSON
    let event_json = serde_json::to_string(event)
        .context("Failed to serialize event to JSON")?;

    info!("Executing handler: {}", handler_cmd);

    // Parse the command and arguments
    let parts: Vec<&str> = handler_cmd.split_whitespace().collect();
    if parts.is_empty() {
        bail!("MESSAGE_HANDLER_CMD is empty");
    }

    let program = parts[0];
    let args = &parts[1..];

    // Spawn the handler process with stdin/stdout/stderr piped.
    // The subject travels via the environment because stdin carries only
    // the raw event body.
    let mut child = Command::new(program)
        .args(args)
        .env("NATS_SUBJECT", subject)
        .env("NATS_MSG_ID", event_id)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn handler command: {}", handler_cmd))?;

    // Write the event JSON to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(event_json.as_bytes()).await
            .context("Failed to write event to handler stdin")?;
        stdin.write_all(b"\n").await
            .context("Failed to write newline to handler stdin")?;
        // Close stdin to signal EOF
        drop(stdin);
    }

    // Wait for the process to complete
    let output = child.wait_with_output().await
        .context("Failed to wait for handler process")?;

    // Check exit code
    if output.status.success() {
        // Log stdout if present
        if !output.stdout.is_empty() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            info!("Handler stdout: {}", stdout_str.trim());
        }
        info!("Handler completed successfully");
        Ok(())
    } else {
        // Log stderr if present
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        error!(
            "Handler failed with exit code {}: {}",
            exit_code,
            stderr_str.trim()
        );

        bail!(
            "Handler exited with code {}: {}",
            exit_code,
            stderr_str.trim()
        );
    }
}
