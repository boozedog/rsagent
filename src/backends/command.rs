use std::process::Command;

use crate::error::{Result, RsagentError};

const MAX_COMMAND_OUTPUT: usize = 16_000;

pub(crate) fn run_command(tool: &str, program: &str, args: &[&str]) -> Result<String> {
	let output = Command::new(program)
		.args(args)
		.output()
		.map_err(|e| RsagentError::tool(tool, format!("failed to run `{program}`: {e}")))?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	let mut text = String::new();
	if !stdout.trim().is_empty() {
		text.push_str(stdout.trim());
	}
	if !stderr.trim().is_empty() {
		if !text.is_empty() {
			text.push('\n');
		}
		text.push_str(stderr.trim());
	}

	if !output.status.success() && text.is_empty() {
		return Err(RsagentError::tool(
			tool,
			format!("`{program}` exited with {}", output.status),
		));
	}

	if text.len() > MAX_COMMAND_OUTPUT {
		text.truncate(MAX_COMMAND_OUTPUT);
		text.push_str("\n[truncated]");
	}

	if text.is_empty() {
		text = format!("`{program}` completed successfully with no output");
	}

	Ok(text)
}
