use std::collections::HashMap;

use serde_json::Value;

use super::command::run_command;
use crate::backends::parse_input;
use crate::error::{Result, RsagentError};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct Fail2banInput {
	/// Optional jail name, e.g. `sshd`.
	jail: Option<String>,
}

pub fn status(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: Fail2banInput = parse_input("fail2ban.status", input)?;

	if let Some(jail) = call.jail.as_deref() {
		validate_jail(params, jail)?;
		return run_command("fail2ban.status", "fail2ban-client", &["status", jail]);
	}

	run_command("fail2ban.status", "fail2ban-client", &["status"])
}

pub fn validate_fail2ban(params: &HashMap<String, toml::Value>) -> Result<()> {
	let _ = params;
	Ok(())
}

fn validate_jail(params: &HashMap<String, toml::Value>, jail: &str) -> Result<()> {
	let Some(raw) = params.get("allowed_jails") else {
		return Ok(());
	};

	let jails = raw.as_array().ok_or_else(|| {
		RsagentError::config("params.allowed_jails must be an array of strings")
	})?;

	if jails.iter().any(|entry| entry.as_str() == Some(jail)) {
		Ok(())
	} else {
		Err(RsagentError::tool(
			"fail2ban.status",
			format!("jail `{jail}` is not in allowed_jails"),
		))
	}
}
