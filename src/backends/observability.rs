use std::collections::HashMap;

use serde_json::Value;
use urlencoding::encode;

use super::http_client::http_get;
use super::url_util::{ensure_allowed_prefixes, param_base_url};
use crate::backends::{param_u32, parse_input};
use crate::error::{Result, RsagentError};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct PrometheusInput {
	/// PromQL expression, e.g. `up == 0`.
	query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct LokiInput {
	/// LogQL expression.
	query: String,
	/// Maximum log lines (capped by config `max_lines`).
	limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct AlertmanagerInput {
	/// Optional alertname substring filter.
	alertname: Option<String>,
	/// Optional severity label filter (warning, critical, etc.).
	severity: Option<String>,
}

pub fn prometheus_query(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: PrometheusInput = parse_input("prometheus.query", input)?;
	let query = call.query.trim();
	if query.is_empty() {
		return Err(RsagentError::tool("prometheus.query", "query must not be empty"));
	}

	ensure_allowed_prefixes("prometheus.query", query, params, "allowed_query_prefixes")?;

	let base_url = param_base_url(params, "base_url", "http://127.0.0.1:9090")?;
	let url = format!("{base_url}/api/v1/query?query={}", encode(query));

	let body = http_get(&url)?;
	format_prometheus_response(&body)
}

pub fn loki_query(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: LokiInput = parse_input("loki.query", input)?;
	let query = call.query.trim();
	if query.is_empty() {
		return Err(RsagentError::tool("loki.query", "query must not be empty"));
	}

	ensure_allowed_prefixes("loki.query", query, params, "allowed_query_prefixes")?;

	let base_url = param_base_url(params, "base_url", "http://127.0.0.1:3030")?;
	let max_lines = param_u32(params, "max_lines", 200).min(500);
	let limit = call.limit.unwrap_or(max_lines).min(max_lines);

	let since = params
		.get("since")
		.and_then(|v| v.as_str())
		.unwrap_or("1h");

	let end = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or(0);
	let start = end.saturating_sub(parse_since_ns(since));

	let url = format!(
		"{base_url}/loki/api/v1/query_range?query={}&limit={limit}&start={start}&end={end}",
		encode(query)
	);

	http_get(&url)
}

pub fn alertmanager_alerts(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: AlertmanagerInput = parse_input("alertmanager.alerts", input)?;
	let base_url = param_base_url(params, "base_url", "http://127.0.0.1:9093")?;
	let url = format!("{base_url}/api/v2/alerts");

	let body = http_get(&url)?;
	filter_alertmanager_alerts(&body, &call)
}

pub fn validate_prometheus(params: &HashMap<String, toml::Value>) -> Result<()> {
	param_base_url(params, "base_url", "http://127.0.0.1:9090")?;
	Ok(())
}

pub fn validate_loki(params: &HashMap<String, toml::Value>) -> Result<()> {
	param_base_url(params, "base_url", "http://127.0.0.1:3030")?;
	Ok(())
}

pub fn validate_alertmanager(params: &HashMap<String, toml::Value>) -> Result<()> {
	param_base_url(params, "base_url", "http://127.0.0.1:9093")?;
	Ok(())
}

fn format_prometheus_response(raw: &str) -> Result<String> {
	let body = raw
		.split_once('\n')
		.map(|(_, b)| b)
		.unwrap_or(raw);

	let json: Value = serde_json::from_str(body)
		.map_err(|e| RsagentError::tool("prometheus.query", format!("invalid JSON: {e}")))?;

	if json["status"] != "success" {
		return Ok(body.to_string());
	}

	let results = &json["data"]["result"];
	if !results.is_array() || results.as_array().is_some_and(|a| a.is_empty()) {
		return Ok("Prometheus: no series matched the query".into());
	}

	let mut out = String::from("Prometheus results:\n");
	if let Some(series) = results.as_array() {
		for (idx, item) in series.iter().take(50).enumerate() {
			let metric = item["metric"]
				.as_object()
				.map(|m| {
					m.iter()
						.map(|(k, v)| format!("{k}={v}"))
						.collect::<Vec<_>>()
						.join(", ")
				})
				.unwrap_or_default();
			let value = item["value"]
				.as_array()
				.and_then(|v| v.get(1))
				.map(|v| v.to_string())
				.unwrap_or_else(|| "?".into());
			out.push_str(&format!("{}. {{{metric}}} => {value}\n", idx + 1));
		}
		if series.len() > 50 {
			out.push_str(&format!("\n[showing 50 of {} series]", series.len()));
		}
	}

	Ok(out)
}

fn filter_alertmanager_alerts(raw: &str, filter: &AlertmanagerInput) -> Result<String> {
	let body = raw
		.split_once('\n')
		.map(|(_, b)| b)
		.unwrap_or(raw);

	let alerts: Vec<Value> = serde_json::from_str(body).map_err(|e| {
		RsagentError::tool("alertmanager.alerts", format!("invalid alerts JSON: {e}"))
	})?;

	if alerts.is_empty() {
		return Ok("Alertmanager: no firing alerts".into());
	}

	let mut out = String::from("Firing alerts:\n");
	let mut shown = 0usize;

	for alert in alerts {
		let labels = alert["labels"].as_object();
		let annotations = alert["annotations"].as_object();

		let alertname = labels
			.and_then(|l| l.get("alertname"))
			.and_then(|v| v.as_str())
			.unwrap_or("unknown");
		let severity = labels
			.and_then(|l| l.get("severity"))
			.and_then(|v| v.as_str())
			.unwrap_or("unknown");
		let instance = labels
			.and_then(|l| l.get("instance"))
			.or_else(|| labels.and_then(|l| l.get("host")))
			.and_then(|v| v.as_str())
			.unwrap_or("-");

		if let Some(name_filter) = filter.alertname.as_deref() {
			if !alertname.contains(name_filter) {
				continue;
			}
		}
		if let Some(sev_filter) = filter.severity.as_deref() {
			if severity != sev_filter {
				continue;
			}
		}

		shown += 1;
		let summary = annotations
			.and_then(|a| a.get("summary"))
			.and_then(|v| v.as_str())
			.unwrap_or("");
		let impact = annotations
			.and_then(|a| a.get("impact"))
			.and_then(|v| v.as_str())
			.unwrap_or("");
		out.push_str(&format!(
			"- {alertname} ({severity}) @ {instance}\n  summary: {summary}\n  impact: {impact}\n"
		));

		if shown >= 50 {
			out.push_str("\n[truncated to 50 alerts]");
			break;
		}
	}

	if shown == 0 {
		return Ok("Alertmanager: no alerts matched the filter".into());
	}

	Ok(out)
}

fn parse_since_ns(since: &str) -> u128 {
	let since = since.trim();
	if let Some(hours) = since.strip_prefix('-').and_then(|s| s.strip_suffix('h')) {
		if let Ok(h) = hours.parse::<u128>() {
			return h * 3_600_000_000_000;
		}
	}
	if let Some(mins) = since.strip_prefix('-').and_then(|s| s.strip_suffix('m')) {
		if let Ok(m) = mins.parse::<u128>() {
			return m * 60_000_000_000;
		}
	}
	3_600_000_000_000
}
