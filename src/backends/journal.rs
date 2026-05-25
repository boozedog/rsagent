use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
fn now_usec() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_micros() as u64
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
fn parse_since_us(since: &str) -> Result<u64> {
	if !since.starts_with('-') {
		return Err(RsagentError::tool(
			"journal.query",
			format!("unsupported since value `{since}` (use e.g. -1h, -30m, -24h)"),
		));
	}

	let spec = &since[1..];
	let (value, multiplier) = if let Some(hours) = spec.strip_suffix('h') {
		(
			hours.parse::<u64>().map_err(|_| {
				RsagentError::tool("journal.query", format!("invalid since value `{since}`"))
			})?,
			3_600_u64,
		)
	} else if let Some(minutes) = spec.strip_suffix('m') {
		(
			minutes.parse::<u64>().map_err(|_| {
				RsagentError::tool("journal.query", format!("invalid since value `{since}`"))
			})?,
			60_u64,
		)
	} else if let Some(days) = spec.strip_suffix('d') {
		(
			days.parse::<u64>().map_err(|_| {
				RsagentError::tool("journal.query", format!("invalid since value `{since}`"))
			})?,
			86_400_u64,
		)
	} else {
		return Err(RsagentError::tool(
			"journal.query",
			format!("unsupported since suffix in `{since}` (use h, m, or d)"),
		));
	};

	Ok(now_usec().saturating_sub(value * multiplier * 1_000_000))
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
fn query_linux(unit: &str, since: &str, priority: Option<&str>, lines: u32) -> Result<String> {
	use journald_query::{query_journal, Query};

	let unit_field = if unit.ends_with(".service") {
		unit.to_string()
	} else {
		format!("{unit}.service")
	};

	let start = parse_since_us(since)?;
	let end = now_usec();
	let query = Query::new(start, end).unit(unit_field.clone());

	let journal_dirs = [Path::new("/var/log/journal"), Path::new("/run/log/journal")];
	let mut entries = Vec::new();
	for dir in journal_dirs {
		if !dir.exists() {
			continue;
		}
		entries = query_journal(dir, query.clone()).map_err(|e| {
			RsagentError::tool("journal.query", format!("journal query failed: {e}"))
		})?;
		if !entries.is_empty() {
			break;
		}
	}

	if let Some(priority) = priority {
		let _ = priority;
		// journald-query 0.1 does not expose PRIORITY; callers still pass it for forward compatibility.
	}

	if entries.is_empty() {
		return Ok(format!("No journal entries for `{unit_field}` since {since}"));
	}

	if entries.len() > lines as usize {
		entries.truncate(lines as usize);
	}

	let mut out = String::new();
	for entry in entries {
		out.push_str(&format!(
			"[{}] {}\n",
			entry.timestamp_utc, entry.message
		));
	}

	Ok(out)
}
