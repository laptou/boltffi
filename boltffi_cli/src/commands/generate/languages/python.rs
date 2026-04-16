use boltffi_bindgen::render::python::{PythonEmitter, PythonLowerError, PythonLowerer};

use crate::cli::{CliError, Result};
use crate::commands::generate::generator::{GenerateRequest, LanguageGenerator, ScanPointerWidth};
use crate::config::Target;

pub struct PythonGenerator;

impl PythonGenerator {
    pub(crate) fn generate_from_source_directory(
        config: &crate::config::Config,
        output_override: Option<std::path::PathBuf>,
        source_directory: &std::path::Path,
        crate_name: &str,
    ) -> Result<()> {
        let request = GenerateRequest::new(
            config,
            output_override,
            crate::commands::generate::generator::SourceCrate::new(source_directory, crate_name),
        );

        Self::generate(&request)
    }
}

impl LanguageGenerator for PythonGenerator {
    const TARGET: Target = Target::Python;

    fn generate(request: &GenerateRequest<'_>) -> Result<()> {
        if !request.config().is_python_enabled() {
            return Err(CliError::CommandFailed {
                command: "targets.python.enabled = false".to_string(),
                status: None,
            });
        }

        let output_directory = request
            .output_override()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| request.config().python_output());

        request.ensure_output_directory(&output_directory)?;

        let lowered_crate = request.lowered_crate(ScanPointerWidth::Host)?;
        let module_name = request.config().python_module_name();
        let python_module = PythonLowerer::new(
            &lowered_crate.ffi_contract,
            &lowered_crate.abi_contract,
            &module_name,
            &request.config().package.name,
            request.config().package_version(),
            &request.config().crate_artifact_name(),
        )
        .lower()
        .map_err(|error| match error {
            PythonLowerError::TopLevelFunctionNameCollision { .. }
            | PythonLowerError::ParameterNameCollision { .. } => CliError::CommandFailed {
                command: format!("generate python: {error}"),
                status: None,
            },
        })?;
        let python_sources = PythonEmitter::emit(&python_module);

        python_sources.files.iter().try_for_each(|output_file| {
            let output_path = output_directory.join(&output_file.relative_path);

            if let Some(parent_directory) = output_path.parent() {
                request.ensure_output_directory(parent_directory)?;
            }

            request.write_output(&output_path, &output_file.contents)
        })
    }
}
