use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonOutputFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonPackageSources {
    pub files: Vec<PythonOutputFile>,
}
