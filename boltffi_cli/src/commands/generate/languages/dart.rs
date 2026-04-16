use boltffi_bindgen::render::dart::{DartEmitter, DartLowerer};

use crate::cli::{CliError, Result};
use crate::commands::generate::generator::{GenerateRequest, LanguageGenerator, ScanPointerWidth};
use crate::config::Target;

pub struct DartGenerator;

impl LanguageGenerator for DartGenerator {
    const TARGET: Target = Target::Dart;

    fn generate(request: &GenerateRequest<'_>) -> Result<()> {
        if !request.config().is_dart_enabled() {
            return Err(CliError::CommandFailed {
                command: "targets.dart.enabled = false".to_string(),
                status: None,
            });
        }

        let output_directory = request
            .output_override()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| request.config().targets.dart.output.clone());

        request.ensure_output_directory(&output_directory)?;

        let lowered_crate = request.lowered_crate(ScanPointerWidth::Fixed(32))?;
        let dart_library = DartLowerer::new(
            &lowered_crate.ffi_contract,
            &lowered_crate.abi_contract,
            &request.config().package.name,
        )
        .library();
        let dart_source = DartEmitter::emit(&dart_library);
        let output_path = output_directory.join(format!("{}.dart", request.config().package.name));

        request.write_output(&output_path, dart_source)
    }
}
