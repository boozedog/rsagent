use std::collections::HashMap;

use serde_json::Value;

use super::command::run_command;
use crate::backends::{ensure_allowed_strings, parse_input};
use crate::error::Result;

use super::EmptyInput;

pub fn pool_status(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let _: EmptyInput = parse_input("zfs.pool_status", input)?;
	let allowed = ensure_allowed_strings(params, "pools")?;

	if allowed.is_empty() {
		return run_command("zfs.pool_status", "zpool", &["status", "-x"]);
	}

	let mut out = String::new();
	for pool in allowed {
		let chunk = run_command("zfs.pool_status", "zpool", &["status", "-x", &pool])?;
		out.push_str(&format!("=== {pool} ===\n{chunk}\n\n"));
	}
	Ok(out)
}

pub fn validate_zfs(params: &HashMap<String, toml::Value>) -> Result<()> {
	let _ = ensure_allowed_strings(params, "pools")?;
	Ok(())
}
