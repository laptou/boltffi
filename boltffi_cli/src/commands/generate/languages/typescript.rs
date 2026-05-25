use boltffi_bindgen::render::typescript::{
    TypeScriptEmitter, TypeScriptLowerError, TypeScriptLowerer,
};

use crate::cli::{CliError, Result};
use crate::commands::generate::generator::{GenerateRequest, LanguageGenerator, ScanPointerWidth};
use crate::config::{Target, WasmNpmTarget};

pub struct TypeScriptGenerator;

impl LanguageGenerator for TypeScriptGenerator {
    const TARGET: Target = Target::TypeScript;

    fn generate(request: &GenerateRequest<'_>) -> Result<()> {
        if !request.config().is_wasm_enabled() {
            return Err(CliError::CommandFailed {
                command: "targets.wasm.enabled = false".to_string(),
                status: None,
            });
        }

        let output_directory = request
            .output_override()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| request.config().wasm_typescript_output());
        let module_name = request.config().wasm_typescript_module_name();
        let browser_output_path = output_directory.join(format!("{module_name}.ts"));
        let node_output_path = output_directory.join(format!("{module_name}_node.ts"));

        request.ensure_output_directory(&output_directory)?;

        let lowered_crate = request.lowered_crate(
            ScanPointerWidth::Fixed(32),
            Some(request.config().wasm_scan_cfg_context()),
        )?;

        let type_script_module = TypeScriptLowerer::new(
            &lowered_crate.ffi_contract,
            &lowered_crate.abi_contract,
            request.source_crate().crate_name().to_string(),
            request.config().typescript_experimental(),
        )
        .lower()
        .map_err(|error| match error {
            TypeScriptLowerError::ValueTypeMemberNameCollision { .. }
            | TypeScriptLowerError::TopLevelFunctionNameCollision { .. } => {
                CliError::CommandFailed {
                    command: format!("generate typescript: {error}"),
                    status: None,
                }
            }
        })?;
        let runtime_package = request.config().wasm_runtime_package();
        let emits_node_bundle = request
            .config()
            .wasm_npm_targets()
            .iter()
            .any(|target| matches!(target, WasmNpmTarget::Nodejs));

        let browser_source = TypeScriptEmitter::emit(&type_script_module).replacen(
            "from \"@boltffi/runtime\"",
            &format!("from \"{}\"", runtime_package),
            1,
        );

        request.write_output(&browser_output_path, browser_source)?;

        if emits_node_bundle {
            let node_source = TypeScriptEmitter::emit_node(&type_script_module, &module_name)
                .replacen(
                    "from \"@boltffi/runtime\"",
                    &format!("from \"{}\"", runtime_package),
                    1,
                );
            request.write_output(&node_output_path, node_source)?;
        } else if node_output_path.exists() {
            std::fs::remove_file(&node_output_path).map_err(|source| CliError::WriteFailed {
                path: node_output_path.clone(),
                source,
            })?;
        }

        Ok(())
    }
}
