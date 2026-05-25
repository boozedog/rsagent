use std::io::{self, Write};

use aisdk::core::utils::step_count_is;
use aisdk::core::{DynamicModel, LanguageModelRequest, Message, Messages, UserMessage};
use aisdk::providers::FireworksAi;

use crate::config::Config;
use crate::error::Result;
use crate::registry::ToolRegistry;

pub struct Session {
	model: FireworksAi<DynamicModel>,
	tools: Vec<aisdk::core::tools::Tool>,
	system_prompt: String,
	max_steps: u32,
	messages: Messages,
}

impl Session {
	pub fn new(config: &Config) -> Result<Self> {
		let api_key = config.api_key()?;
		let registry = ToolRegistry::from_config(config)?;

		let model = FireworksAi::<DynamicModel>::builder()
			.model_name(config.llm.model.clone())
			.base_url(config.llm.base_url.clone())
			.api_key(api_key)
			.build()
			.map_err(|e| crate::error::RsagentError::Other(e.into()))?;

		Ok(Self {
			model,
			tools: registry.aisdk_tools(),
			system_prompt: config.agent.system_prompt.clone(),
			max_steps: config.agent.max_steps,
			messages: Vec::new(),
		})
	}

	pub fn clear(&mut self) {
		self.messages.clear();
	}

	pub async fn turn(&mut self, prompt: &str) -> Result<String> {
		self.messages
			.push(Message::User(UserMessage::new(prompt)));

		let mut builder = LanguageModelRequest::builder()
			.model(self.model.clone())
			.system(self.system_prompt.clone())
			.messages(self.messages.clone())
			.stop_when(step_count_is(self.max_steps as usize));

		for tool in &self.tools {
			builder = builder.with_tool(tool.clone());
		}

		let response = builder
			.build()
			.generate_text()
			.await
			.map_err(|e| crate::error::RsagentError::Other(e.into()))?;

		self.messages = response.messages();

		Ok(response.text().unwrap_or_default())
	}
}

pub async fn ask(config: &Config, prompt: &str) -> Result<String> {
	let mut session = Session::new(config)?;
	session.turn(prompt).await
}

pub async fn interactive(config: &Config) -> Result<()> {
	let mut session = Session::new(config)?;

	eprintln!("rsagent interactive (/exit or Ctrl-D to quit, /clear to reset history)");
	loop {
		print!("> ");
		let _ = io::stdout().flush();

		let mut line = String::new();
		let bytes = io::stdin()
			.read_line(&mut line)
			.map_err(|e| crate::error::RsagentError::Other(e.into()))?;
		if bytes == 0 {
			break;
		}

		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		match line {
			"/exit" | "/quit" => break,
			"/clear" => {
				session.clear();
				eprintln!("conversation cleared");
				continue;
			}
			_ => match session.turn(line).await {
				Ok(answer) => println!("{answer}\n"),
				Err(err) => eprintln!("error: {err}\n"),
			},
		}
	}

	Ok(())
}
