use std::collections::HashMap;

use serde_json::Value;

#[cfg(feature = "docker")]
use crate::backends::block_on_tool;
use crate::backends::parse_input;
use crate::error::{Result, RsagentError};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[cfg_attr(not(feature = "docker"), allow(dead_code))]
pub(super) struct DockerListInput {
	/// Optional name substring filter (must match configured prefix allowlist when set).
	filter: Option<String>,
}

pub fn list(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: DockerListInput = parse_input("docker.list", input)?;

	#[cfg(feature = "docker")]
	{
		return list_docker(params, call.filter.as_deref());
	}

	#[cfg(not(feature = "docker"))]
	{
		let _ = (params, call);
		Err(RsagentError::tool(
			"docker.list",
			"docker support requires the `docker` build feature",
		))
	}
}

pub fn validate_docker(_params: &HashMap<String, toml::Value>) -> Result<()> {
	Ok(())
}

#[cfg(feature = "docker")]
fn list_docker(params: &HashMap<String, toml::Value>, filter: Option<&str>) -> Result<String> {
	use bollard::Docker;
	use crate::backends::param_string_opt;

	if let Some(filter) = filter {
		validate_name_prefix(params, filter)?;
	}

	let prefix = param_string_opt(params, "name_prefix");

	block_on_tool(list_docker_async(prefix.as_deref(), filter))
}

#[cfg(feature = "docker")]
async fn list_docker_async(
	name_prefix: Option<&str>,
	filter: Option<&str>,
) -> Result<String> {
	use bollard::container::ListContainersOptions;
	use bollard::Docker;

	let docker = Docker::connect_with_local_defaults()
		.map_err(|e| RsagentError::tool("docker.list", e.to_string()))?;

	let containers = docker
		.list_containers(Some(ListContainersOptions::<String> {
			all: true,
			..Default::default()
		}))
		.await
		.map_err(|e| RsagentError::tool("docker.list", e.to_string()))?;

	let mut out = String::from("Containers:\n");
	let mut shown = 0usize;

	for container in containers {
		let names: Vec<String> = container
			.names
			.unwrap_or_default()
			.into_iter()
			.map(|n| n.trim_start_matches('/').to_string())
			.collect();
		let primary = names.first().cloned().unwrap_or_else(|| "?".into());

		if let Some(prefix) = name_prefix {
			if !primary.starts_with(prefix) {
				continue;
			}
		}
		if let Some(filter) = filter {
			if !primary.contains(filter) {
				continue;
			}
		}

		shown += 1;
		let state = container.state.unwrap_or_else(|| "unknown".into());
		let status = container.status.unwrap_or_else(|| "unknown".into());
		let image = container.image.unwrap_or_else(|| "unknown".into());
		out.push_str(&format!("- {primary} ({state}/{status}) image={image}\n"));

		if shown >= 100 {
			out.push_str("\n[truncated to 100 containers]");
			break;
		}
	}

	if shown == 0 {
		return Ok("No matching containers".into());
	}

	Ok(out)
}

#[cfg(feature = "docker")]
fn validate_name_prefix(params: &HashMap<String, toml::Value>, filter: &str) -> Result<()> {
	use crate::backends::param_string_opt;
	let Some(prefix) = param_string_opt(params, "name_prefix") else {
		return Ok(());
	};

	if filter.starts_with(&prefix) {
		Ok(())
	} else {
		Err(RsagentError::tool(
			"docker.list",
			format!("filter must start with configured name_prefix `{prefix}`"),
		))
	}
}
