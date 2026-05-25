use aisdk::core::tools::{Tool, ToolExecute};
use schemars::Schema;

use crate::backends::{input_schema, run_tool, validate_tool};
use crate::config::{Config, ToolConfig};
use crate::error::{Result, RsagentError};

pub struct ToolRegistry {
	tools: Vec<Tool>,
}

impl ToolRegistry {
	pub fn from_config(config: &Config) -> Result<Self> {
		let mut tools = Vec::new();

		for tool_cfg in config.enabled_tools() {
			validate_tool(tool_cfg)?;
			tools.push(build_tool(tool_cfg)?);
		}

		if tools.is_empty() {
			return Err(RsagentError::config(
				"no enabled tools in config; add [[tools]] entries with enabled = true",
			));
		}

		Ok(Self { tools })
	}

	pub fn aisdk_tools(self) -> Vec<Tool> {
		self.tools
	}

	pub fn list(config: &Config) -> Vec<(&str, &str, &str)> {
		config
			.enabled_tools()
			.map(|t| (t.name.as_str(), t.kind.as_str(), t.description.as_str()))
			.collect()
	}
}

fn build_tool(tool_cfg: &ToolConfig) -> Result<Tool> {
	let schema: Schema = input_schema(tool_cfg);
	let name = tool_cfg.name.clone();
	let description = tool_cfg.description.clone();
	let kind = tool_cfg.kind.clone();

	let tool_name = tool_cfg.name.clone();

	let tool_config = ToolConfig {
		name: tool_cfg.name.clone(),
		description: tool_cfg.description.clone(),
		kind,
		enabled: true,
		params: tool_cfg.params.clone(),
	};

	let execute = ToolExecute::new(Box::new(move |input| {
		if input.as_object().is_some_and(|m| m.is_empty()) {
			// Allow models that send {} for tools with optional input.
		}

		run_tool(&tool_config, input).map_err(|e| e.to_string())
	}));

	Tool::builder()
		.name(name)
		.description(description)
		.input_schema(schema)
		.execute(execute)
		.build()
		.map_err(|e| RsagentError::config(format!("tool `{tool_name}`: {e}")))
}
