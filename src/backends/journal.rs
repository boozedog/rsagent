use std::collections::HashMap;

use serde_json::Value;

use super::JournalInput;
use crate::backends::{param_string, param_u32, parse_input};
use crate::error::{Result, RsagentError};

pub fn query(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: JournalInput = parse_input("journal.query", input)?;
	let configured_unit = param_string(params, "unit")?;
	let unit = call.unit.unwrap_or(configured_unit);

	let max_lines = param_u32(params, "max_lines", 100).min(500);
	let lines = call.lines.unwrap_or(max_lines).min(max_lines);
	let since = params
		.get("since")
		.and_then(|v| v.as_str())
		.unwrap_or("-1h");
	let priority = params.get("priority").and_then(|v| v.as_str());

	#[cfg(all(target_os = "linux", feature = "systemd"))]
	{
		return query_linux(&unit, since, priority, lines);
	}

	#[cfg(not(all(target_os = "linux", feature = "systemd")))]
	{
		let _ = (unit, since, priority, lines);
		Err(RsagentError::tool(
			"journal.query",
			"journal support requires Linux and the `systemd` build feature",
		))
	}
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
fn query_linux(unit: &str, since: &str, priority: Option<&str>, lines: u32) -> Result<String> {
	use journald_query::{query_journal, Query};

	let unit_field = if unit.ends_with(".service") {
		unit.to_string()
	} else {
		format!("{unit}.service")
	};

	let mut query = Query::new().since(since).limit(lines as usize);
	query = query.match_field("_SYSTEMD_UNIT", &unit_field);

	if let Some(priority) = priority {
		query = query.match_field("PRIORITY", priority);
	}

	let entries = query_journal(query).map_err(|e| {
		RsagentError::tool("journal.query", format!("journal query failed: {e}"))
	})?;

	if entries.is_empty() {
		return Ok(format!("No journal entries for `{unit_field}` since {since}"));
	}

	let mut out = String::new();
	for entry in entries {
		out.push_str(&format!(
			"[{}] {}\n",
			entry.timestamp, entry.message
		));
	}

	Ok(out)
}
