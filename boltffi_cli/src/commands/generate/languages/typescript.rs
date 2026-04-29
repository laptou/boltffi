use std::path::Path;

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

        let mut type_script_module = TypeScriptLowerer::new(
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
        if let Some(glue) = read_wasm_bindgen_glue_marker(&output_directory, &module_name) {
            type_script_module.wasm_bindgen_glue = Some(glue);
        }
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

/// `boltffi pack wasm` writes `{module}.boltffi.json` when wasm-bindgen glue is present.
fn read_wasm_bindgen_glue_marker(output_dir: &Path, module_name: &str) -> Option<String> {
    let path = output_dir.join(format!("{module_name}.boltffi.json"));
    let data = std::fs::read(path).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&data).ok()?;
    v.get("wasm_bindgen_glue")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}
