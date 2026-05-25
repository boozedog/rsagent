use std::collections::HashMap;

use serde_json::Value;

use super::command::run_command;
use crate::backends::{parse_input, param_string};
use crate::error::Result;

use super::EmptyInput;

pub fn status(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let _: EmptyInput = parse_input("gluster.status", input)?;
	let volume = param_string(params, "volume")?;
	run_command(
		"gluster.status",
		"gluster",
		&["volume", "status", &volume],
	)
}

pub fn validate_gluster(params: &HashMap<String, toml::Value>) -> Result<()> {
	param_string(params, "volume")?;
	Ok(())
}
