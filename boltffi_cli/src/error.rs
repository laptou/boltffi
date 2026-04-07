use crate::config::ConfigError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("config error: {0}")]
    Config(Box<ConfigError>),

    #[error("config file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("no built libraries found for {platform}")]
    NoLibrariesFound { platform: String },

    #[error("missing built libraries for {platform}: {targets:?}")]
    MissingBuiltLibraries {
        platform: String,
        targets: Vec<String>,
    },

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

    #[error("xcframework creation failed")]
    XcframeworkFailed { source: std::io::Error },

    #[error("lipo failed for simulator fat library")]
    LipoFailed { source: std::io::Error },

    #[error("zip creation failed")]
    ZipFailed { source: std::io::Error },

    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("android ndk not found (set ANDROID_NDK_HOME or ANDROID_HOME/ANDROID_SDK_ROOT)")]
    AndroidNdkNotFound,

    #[error("invalid android ndk at {path}")]
    AndroidNdkInvalid { path: PathBuf },

    #[error("android ndk toolchain not found at {path}")]
    AndroidToolchainNotFound { path: PathBuf },

    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("verification error: {0}")]
    VerifyError(String),

    #[error("build failed for targets: {targets:?}")]
    BuildFailed { targets: Vec<String> },
}

impl From<ConfigError> for CliError {
    fn from(error: ConfigError) -> Self {
        Self::Config(Box::new(error))
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
