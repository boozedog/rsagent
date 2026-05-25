mod command;
mod docker;
mod fail2ban;
mod gluster;
mod host;
mod http_client;
mod http_health;
mod journal;
mod observability;
mod systemd;
mod tailscale;
mod url_util;
mod zfs;

use std::collections::HashMap;

use schemars::JsonSchema;
use schemars::Schema;
use serde::Deserialize;
use serde_json::Value;

use crate::config::ToolConfig;
use crate::error::{Result, RsagentError};

const MAX_OUTPUT_CHARS: usize = 16_000;

pub fn run_tool(tool: &ToolConfig, input: Value) -> Result<String> {
	let output = match tool.kind.as_str() {
		"host.memory" => host::memory(&tool.params, input),
		"host.disk" => host::disk(&tool.params, input),
		"host.load" => host::load(&tool.params, input),
		"host.nixos_revision" => host::nixos_revision(&tool.params, input),
		"systemd.unit_status" => systemd::unit_status(&tool.params, input),
		"systemd.list_units" => systemd::list_units(&tool.params, input),
		"journal.query" => journal::query(&tool.params, input),
		"prometheus.query" => observability::prometheus_query(&tool.params, input),
		"loki.query" => observability::loki_query(&tool.params, input),
		"alertmanager.alerts" => observability::alertmanager_alerts(&tool.params, input),
		"http.health" => http_health::health(&tool.params, input),
		"fail2ban.status" => fail2ban::status(&tool.params, input),
		"tailscale.status" => tailscale::status(&tool.params, input),
		"zfs.pool_status" => zfs::pool_status(&tool.params, input),
		"gluster.status" => gluster::status(&tool.params, input),
		"docker.list" => docker::list(&tool.params, input),
		other => Err(RsagentError::tool(
			&tool.name,
			format!("unknown tool kind `{other}`"),
		)),
	}?;

	Ok(truncate(&output))
}

pub fn input_schema(tool: &ToolConfig) -> Schema {
	match tool.kind.as_str() {
		"host.memory" | "host.load" | "host.nixos_revision" | "tailscale.status" | "zfs.pool_status"
		| "gluster.status" => schemars::schema_for!(EmptyInput),
		"host.disk" => schemars::schema_for!(DiskInput),
		"systemd.unit_status" => schemars::schema_for!(UnitInput),
		"systemd.list_units" => schemars::schema_for!(ListUnitsInput),
		"journal.query" => schemars::schema_for!(JournalInput),
		"prometheus.query" => schemars::schema_for!(observability::PrometheusInput),
		"loki.query" => schemars::schema_for!(observability::LokiInput),
		"alertmanager.alerts" => schemars::schema_for!(observability::AlertmanagerInput),
		"http.health" => schemars::schema_for!(http_health::HttpHealthInput),
		"fail2ban.status" => schemars::schema_for!(fail2ban::Fail2banInput),
		"docker.list" => schemars::schema_for!(docker::DockerListInput),
		_ => schemars::schema_for!(EmptyInput),
	}
}

pub fn validate_tool(tool: &ToolConfig) -> Result<()> {
	match tool.kind.as_str() {
		"host.memory" | "host.load" => Ok(()),
		"host.nixos_revision" => Ok(()),
		"host.disk" => {
			let _mount = param_string(&tool.params, "mount").unwrap_or_else(|_| "/".into());
			Ok(())
		}
		"systemd.unit_status" => {
			ensure_allowed_units(&tool.params)?;
			Ok(())
		}
		"systemd.list_units" => Ok(()),
		"journal.query" => {
			param_string(&tool.params, "unit")?;
			Ok(())
		}
		"prometheus.query" => observability::validate_prometheus(&tool.params),
		"loki.query" => observability::validate_loki(&tool.params),
		"alertmanager.alerts" => observability::validate_alertmanager(&tool.params),
		"http.health" => http_health::validate_health(&tool.params),
		"fail2ban.status" => fail2ban::validate_fail2ban(&tool.params),
		"tailscale.status" => tailscale::validate_tailscale(&tool.params),
		"zfs.pool_status" => zfs::validate_zfs(&tool.params),
		"gluster.status" => gluster::validate_gluster(&tool.params),
		"docker.list" => docker::validate_docker(&tool.params),
		other => Err(RsagentError::config(format!(
			"tool `{}`: unknown kind `{other}`",
			tool.name
		))),
	}
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct EmptyInput {}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct DiskInput {
	/// Mount point to inspect (defaults to `/` from config).
	mount: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct UnitInput {
	/// Systemd unit name, e.g. nginx.service
	unit: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct ListUnitsInput {
	/// Optional substring filter applied to unit names.
	filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct JournalInput {
	/// Override unit from config for this call.
	unit: Option<String>,
	/// Maximum log lines (capped by config `max_lines`).
	lines: Option<u32>,
}

pub(crate) fn parse_input<T: for<'de> Deserialize<'de>>(
	tool_name: &str,
	input: Value,
) -> Result<T> {
	serde_json::from_value(input).map_err(|e| RsagentError::tool(tool_name, e.to_string()))
}

pub(crate) fn param_string(
	params: &HashMap<String, toml::Value>,
	key: &str,
) -> Result<String> {
	params
		.get(key)
		.and_then(|v| v.as_str().map(str::to_string))
		.ok_or_else(|| RsagentError::config(format!("missing params.{key}")))
}

pub(crate) fn param_string_opt(params: &HashMap<String, toml::Value>, key: &str) -> Option<String> {
	params
		.get(key)
		.and_then(|v| v.as_str().map(str::to_string))
}

pub(crate) fn param_u32(params: &HashMap<String, toml::Value>, key: &str, default: u32) -> u32 {
	params
		.get(key)
		.and_then(|v| v.as_integer())
		.and_then(|v| u32::try_from(v).ok())
		.unwrap_or(default)
}

pub(crate) fn ensure_allowed_units(params: &HashMap<String, toml::Value>) -> Result<Vec<String>> {
	let Some(value) = params.get("allowed_units") else {
		return Err(RsagentError::config(
			"systemd.unit_status requires params.allowed_units",
		));
	};

	let units = value.as_array().ok_or_else(|| {
		RsagentError::config("params.allowed_units must be an array of unit names")
	})?;

	let names: Result<Vec<String>> = units
		.iter()
		.map(|entry| {
			entry
				.as_str()
				.map(str::to_string)
				.ok_or_else(|| RsagentError::config("allowed_units entries must be strings"))
		})
		.collect();

	names
}

pub(crate) fn ensure_allowed_strings(
	params: &HashMap<String, toml::Value>,
	key: &str,
) -> Result<Vec<String>> {
	let Some(value) = params.get(key) else {
		return Ok(Vec::new());
	};

	let items = value.as_array().ok_or_else(|| {
		RsagentError::config(format!("params.{key} must be an array of strings"))
	})?;

	items
		.iter()
		.map(|entry| {
			entry
				.as_str()
				.map(str::to_string)
				.ok_or_else(|| RsagentError::config(format!("params.{key} entries must be strings")))
		})
		.collect()
}

pub(crate) fn validate_unit(
	tool_name: &str,
	unit: &str,
	allowed: &[String],
) -> Result<()> {
	if !unit.ends_with(".service")
		&& !unit.ends_with(".timer")
		&& !unit.ends_with(".socket")
		&& !unit.ends_with(".target")
	{
		return Err(RsagentError::tool(
			tool_name,
			"only .service, .timer, .socket, and .target units are allowed",
		));
	}

	if !allowed.iter().any(|u| u == unit) {
		return Err(RsagentError::tool(
			tool_name,
			format!("unit `{unit}` is not in allowed_units"),
		));
	}

	Ok(())
}

fn truncate(text: &str) -> String {
	if text.len() <= MAX_OUTPUT_CHARS {
		return text.to_string();
	}

	format!(
		"{}…\n\n[truncated to {MAX_OUTPUT_CHARS} chars]",
		&text[..MAX_OUTPUT_CHARS]
	)
}
