use thiserror::Error;

#[derive(Debug, Error)]
pub enum RsagentError {
	#[error("configuration: {0}")]
	Config(String),
	#[error("tool `{name}`: {message}")]
	Tool { name: String, message: String },
	#[error(transparent)]
	Other(#[from] anyhow::Error),
}

impl RsagentError {
	pub fn config(msg: impl Into<String>) -> Self {
		Self::Config(msg.into())
	}

	pub fn tool(name: impl Into<String>, message: impl Into<String>) -> Self {
		Self::Tool {
			name: name.into(),
			message: message.into(),
		}
	}
}

pub type Result<T> = std::result::Result<T, RsagentError>;
