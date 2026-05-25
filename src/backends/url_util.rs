use url::Url;

use crate::error::{Result, RsagentError};

pub(crate) fn validate_localhost_url(url: &str) -> Result<()> {
	let parsed = Url::parse(url).map_err(|e| RsagentError::config(format!("invalid URL `{url}`: {e}")))?;

	match parsed.host_str() {
		Some("127.0.0.1") | Some("localhost") | Some("[::1]") => Ok(()),
		Some(host) => Err(RsagentError::config(format!(
			"URL host `{host}` is not allowed; only 127.0.0.1, localhost, and [::1]"
		))),
		None => Err(RsagentError::config(format!(
			"URL `{url}` must include an explicit localhost host"
		))),
	}
}

pub(crate) fn param_base_url(
	params: &std::collections::HashMap<String, toml::Value>,
	key: &str,
	default: &str,
) -> Result<String> {
	let url = super::param_string_opt(params, key)
		.unwrap_or_else(|| default.to_string());
	validate_localhost_url(&url)?;
	Ok(url.trim_end_matches('/').to_string())
}

pub(crate) fn ensure_allowed_prefixes(
	tool_name: &str,
	value: &str,
	params: &std::collections::HashMap<String, toml::Value>,
	param_key: &str,
) -> Result<()> {
	let Some(raw) = params.get(param_key) else {
		return Ok(());
	};

	let prefixes = raw.as_array().ok_or_else(|| {
		RsagentError::config(format!("params.{param_key} must be an array of strings"))
	})?;

	if prefixes.is_empty() {
		return Ok(());
	}

	let allowed = prefixes.iter().any(|entry| {
		entry
			.as_str()
			.is_some_and(|prefix| !prefix.is_empty() && value.starts_with(prefix))
	});

	if allowed {
		Ok(())
	} else {
		Err(RsagentError::tool(
			tool_name,
			format!("query must start with one of the configured {param_key} prefixes"),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn accepts_loopback_hosts() {
		validate_localhost_url("http://127.0.0.1:9090").unwrap();
		validate_localhost_url("http://localhost:9093/api/v2/alerts").unwrap();
	}

	#[test]
	fn rejects_remote_hosts() {
		assert!(validate_localhost_url("http://example.com").is_err());
	}
}
