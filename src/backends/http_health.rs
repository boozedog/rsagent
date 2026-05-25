use std::collections::HashMap;

use serde_json::Value;

use super::http_client::http_get;
use super::url_util::validate_localhost_url;
use crate::backends::parse_input;
use crate::error::{Result, RsagentError};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct HttpHealthInput {
	/// Exact URL from the configured allowlist.
	url: String,
}

pub fn health(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: HttpHealthInput = parse_input("http.health", input)?;
	let allowed = ensure_allowed_urls(params)?;

	if !allowed.iter().any(|url| url == &call.url) {
		return Err(RsagentError::tool(
			"http.health",
			format!("url `{}` is not in allowed_urls", call.url),
		));
	}

	validate_localhost_url(&call.url)?;
	http_get(&call.url)
}

pub fn validate_health(params: &HashMap<String, toml::Value>) -> Result<()> {
	let urls = ensure_allowed_urls(params)?;
	if urls.is_empty() {
		return Err(RsagentError::config(
			"http.health requires non-empty params.allowed_urls",
		));
	}
	for url in urls {
		validate_localhost_url(&url)?;
	}
	Ok(())
}

fn ensure_allowed_urls(params: &HashMap<String, toml::Value>) -> Result<Vec<String>> {
	let value = params.get("allowed_urls").ok_or_else(|| {
		RsagentError::config("http.health requires params.allowed_urls")
	})?;

	let urls = value.as_array().ok_or_else(|| {
		RsagentError::config("params.allowed_urls must be an array of URL strings")
	})?;

	urls
		.iter()
		.map(|entry| {
			entry
				.as_str()
				.map(str::to_string)
				.ok_or_else(|| RsagentError::config("allowed_urls entries must be strings"))
		})
		.collect()
}
