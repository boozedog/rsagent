use std::collections::HashMap;

use serde_json::Value;

use super::command::run_command;
use crate::backends::parse_input;
use crate::error::Result;

use super::EmptyInput;

pub fn status(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let _: EmptyInput = parse_input("tailscale.status", input)?;
	let _ = params;
	run_command("tailscale.status", "tailscale", &["status", "--json"])
}

pub fn validate_tailscale(_params: &HashMap<String, toml::Value>) -> Result<()> {
	Ok(())
}
