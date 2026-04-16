use boltffi_bindgen::ffi_prefix;
use boltffi_bindgen::render::swift::{SwiftEmitter, SwiftLowerer};

use crate::cli::{CliError, Result};
use crate::commands::generate::generator::{
    GenerateRequest, LanguageGenerator, ScanPointerWidth, bindgen_type_mappings,
};
use crate::config::Target;

pub struct SwiftGenerator;

impl SwiftGenerator {
    fn bindings_file_name(library_name: &str) -> String {
        let mut characters = library_name.chars();

        match characters.next() {
            Some(first_character) => {
                format!(
                    "{}{}BoltFFI.swift",
                    first_character.to_uppercase(),
                    characters.as_str()
                )
            }
            None => "BoltFFI.swift".to_string(),
        }
    }
}

impl LanguageGenerator for SwiftGenerator {
    const TARGET: Target = Target::Swift;

    fn generate(request: &GenerateRequest<'_>) -> Result<()> {
        if !request.config().is_apple_enabled() {
            return Err(CliError::CommandFailed {
                command: "targets.apple.enabled = false".to_string(),
                status: None,
            });
        }

        let output_directory = request
            .output_override()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| request.config().apple_swift_output());
        let output_path =
            output_directory.join(Self::bindings_file_name(request.config().library_name()));

        request.ensure_output_directory(&output_directory)?;

        let lowered_crate = request.lowered_crate(ScanPointerWidth::Fixed(64))?;
        let ffi_module_name = request
            .config()
            .apple_swift_ffi_module_name()
            .map(str::to_string)
            .unwrap_or_else(|| format!("{}FFI", request.config().xcframework_name()));
        let type_mappings = bindgen_type_mappings(request.config().swift_type_mappings());
        let swift_module =
            SwiftLowerer::new(&lowered_crate.ffi_contract, &lowered_crate.abi_contract)
                .with_type_mappings(type_mappings)
                .lower();
        let swift_source = SwiftEmitter::with_prefix(ffi_prefix())
            .with_ffi_module(&ffi_module_name)
            .emit(&swift_module);

        request.write_output(&output_path, swift_source)
    }
}
