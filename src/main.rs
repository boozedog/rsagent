mod agent;
mod backends;
mod config;
mod error;
mod registry;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::error::Result;

#[derive(Parser, Debug)]
#[command(name = "rsagent", about = "Config-defined server agent CLI")]
struct Cli {
	/// Path to config.toml (default: RSAGENT_CONFIG or /etc/rsagent/config.toml)
	#[arg(long, global = true)]
	config: Option<PathBuf>,

	/// Run a single prompt and exit (non-interactive)
	#[arg(short = 'p', long)]
	prompt: Option<String>,

	#[command(subcommand)]
	command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// List tools declared in config
	Tools,
	/// Print effective config (secrets redacted)
	Config,
}

#[tokio::main]
async fn main() {
	if let Err(err) = run().await {
		eprintln!("error: {err}");
		std::process::exit(1);
	}
}

async fn run() -> Result<()> {
	let cli = Cli::parse();
	let config = Config::load(cli.config.as_deref())?;

	match cli.command {
		Some(Commands::Tools) => {
			for (name, kind, description) in registry::ToolRegistry::list(&config) {
				println!("{name}\t{kind}\t{description}");
			}
		}
		Some(Commands::Config) => {
			let mut display = config.clone();
			if display.llm.api_key.is_some() {
				display.llm.api_key = Some("***".into());
			}
			println!(
				"{}",
				toml::to_string_pretty(&display).map_err(|e| crate::error::RsagentError::Other(e.into()))?
			);
		}
		None => {
			if let Some(prompt) = cli.prompt {
				let answer = agent::ask(&config, &prompt).await?;
				println!("{answer}");
			} else {
				agent::interactive(&config).await?;
			}
		}
	}

	Ok(())
}
