use std::path::PathBuf;

use crate::config::ConfigError;
use crate::pack::PackError;
use crate::toolchain::AndroidToolchainError;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("config error: {0}")]
    Config(Box<ConfigError>),

    #[error("config file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("command failed: {command}")]
    CommandFailed {
        command: String,
        status: Option<i32>,
    },

    #[error("failed to create directory {path}")]
    CreateDirectoryFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to copy file from {from} to {to}")]
    CopyFailed {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to read file {path}")]
    ReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write file {path}")]
    WriteFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("verification error: {0}")]
    VerifyError(String),

    #[error(transparent)]
    Pack(#[from] PackError),

    #[error(transparent)]
    AndroidToolchain(#[from] AndroidToolchainError),
}

impl From<ConfigError> for CliError {
    fn from(error: ConfigError) -> Self {
        Self::Config(Box::new(error))
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
