use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, RsagentError};

const DEFAULT_CONFIG_PATH: &str = "/etc/rsagent/config.toml";
const DEFAULT_ENV_FILE: &str = "/etc/rsagent/environment";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
	#[serde(default)]
	pub llm: LlmConfig,
	#[serde(default)]
	pub agent: AgentConfig,
	#[serde(default)]
	pub tools: Vec<ToolConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
	/// Fireworks inference API base URL.
	#[serde(default = "default_base_url")]
	pub base_url: String,
	/// Fireworks model or router id.
	#[serde(default = "default_model")]
	pub model: String,
	/// API key override. Prefer FIREWORKS_API_KEY via env or /etc/rsagent/environment.
	#[serde(default)]
	pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
	#[serde(default = "default_system_prompt")]
	pub system_prompt: String,
	#[serde(default = "default_max_steps")]
	pub max_steps: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolConfig {
	pub name: String,
	pub description: String,
	pub kind: String,
	#[serde(default = "default_enabled")]
	pub enabled: bool,
	#[serde(default)]
	pub params: HashMap<String, toml::Value>,
}

impl Default for LlmConfig {
	fn default() -> Self {
		Self {
			base_url: default_base_url(),
			model: default_model(),
			api_key: None,
		}
	}
}

impl Default for AgentConfig {
	fn default() -> Self {
		Self {
			system_prompt: default_system_prompt(),
			max_steps: default_max_steps(),
		}
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			llm: LlmConfig::default(),
			agent: AgentConfig::default(),
			tools: Vec::new(),
		}
	}
}

impl Config {
	pub fn load(path: Option<&Path>) -> Result<Self> {
		let path = path
			.map(PathBuf::from)
			.or_else(default_config_path)
			.ok_or_else(|| {
				RsagentError::config(
					"no config file: pass --config or set RSAGENT_CONFIG or install /etc/rsagent/config.toml",
				)
			})?;

		let raw = fs::read_to_string(&path).map_err(|e| {
			RsagentError::config(format!("failed to read {}: {e}", path.display()))
		})?;

		let config: Config = toml::from_str(&raw).map_err(|e| {
			RsagentError::config(format!("invalid TOML in {}: {e}", path.display()))
		})?;

		config.validate()?;
		Ok(config)
	}

	pub fn api_key(&self) -> Result<String> {
		if let Some(key) = &self.llm.api_key {
			if !key.is_empty() {
				return Ok(key.clone());
			}
		}

		for var in ["FIREWORKS_API_KEY"] {
			if let Ok(value) = std::env::var(var) {
				if !value.is_empty() {
					return Ok(value);
				}
			}
			if let Some(value) = read_env_file(DEFAULT_ENV_FILE, var) {
				if !value.is_empty() {
					return Ok(value);
				}
			}
		}

		Err(RsagentError::config(
			"missing API key: set llm.api_key or FIREWORKS_API_KEY (env or /etc/rsagent/environment)",
		))
	}

	fn validate(&self) -> Result<()> {
		if self.llm.base_url.trim().is_empty() {
			return Err(RsagentError::config("llm.base_url must not be empty"));
		}
		if self.llm.model.trim().is_empty() {
			return Err(RsagentError::config("llm.model must not be empty"));
		}

		let mut names = std::collections::HashSet::new();
		for tool in &self.tools {
			if !tool.enabled {
				continue;
			}
			if !names.insert(tool.name.clone()) {
				return Err(RsagentError::config(format!(
					"duplicate enabled tool name `{}`",
					tool.name
				)));
			}
			if tool.name.trim().is_empty() {
				return Err(RsagentError::config("tool name must not be empty"));
			}
			if tool.kind.trim().is_empty() {
				return Err(RsagentError::config(format!(
					"tool `{}` must declare a kind",
					tool.name
				)));
			}
		}

		Ok(())
	}

	pub fn enabled_tools(&self) -> impl Iterator<Item = &ToolConfig> {
		self.tools.iter().filter(|t| t.enabled)
	}
}

fn default_config_path() -> Option<PathBuf> {
	if let Ok(path) = std::env::var("RSAGENT_CONFIG") {
		if !path.is_empty() {
			return Some(PathBuf::from(path));
		}
	}

	let path = PathBuf::from(DEFAULT_CONFIG_PATH);
	path.exists().then_some(path)
}

fn default_base_url() -> String {
	"https://api.fireworks.ai/inference/v1".into()
}

fn default_model() -> String {
	"accounts/fireworks/routers/kimi-k2p6-turbo".into()
}

fn default_system_prompt() -> String {
	"You are a concise server operations assistant. Use tools to inspect the host. \
	 Never suggest shell commands. Summarize findings clearly."
		.into()
}

fn default_max_steps() -> u32 {
	10
}

fn default_enabled() -> bool {
	true
}

fn read_env_file(path: &str, key: &str) -> Option<String> {
	let raw = std::fs::read_to_string(path).ok()?;
	for line in raw.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let (name, value) = line.split_once('=')?;
		if name == key {
			return Some(value.to_string());
		}
	}
	None
}
