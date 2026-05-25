use std::time::Duration;

use crate::error::{Result, RsagentError};

const HTTP_TIMEOUT_SECS: u64 = 10;
const MAX_BODY_BYTES: usize = 32_768;

pub(crate) fn http_get(url: &str) -> Result<String> {
	let agent = ureq::AgentBuilder::new()
		.timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
		.build();

	let response = agent.get(url).call().map_err(|e| {
		RsagentError::tool("http", format!("GET {url} failed: {e}"))
	})?;

	let status = response.status();
	let mut body = response
		.into_string()
		.map_err(|e| RsagentError::tool("http", format!("read body: {e}")))?;

	if body.len() > MAX_BODY_BYTES {
		body.truncate(MAX_BODY_BYTES);
		body.push_str("\n[body truncated]");
	}

	Ok(format!("HTTP {status}\n{body}"))
}
