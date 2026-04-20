//! run `wasm-bindgen` on cargo-built wasm when it still imports `__wbindgen_placeholder__` shims,
//! then patch the web glue so `env` imports resolve via `@boltffi/runtime` instead of bare `"env"`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use wasmparser::{Parser, Payload};

use crate::cli::{CliError, Result};
use crate::config::Config;

/// wasm built with wasm-bindgen deps lists these module names until `wasm-bindgen` rewrites imports.
pub(crate) fn wasm_has_wasm_bindgen_placeholder_imports(wasm_path: &Path) -> Result<bool> {
    let bytes = fs::read(wasm_path).map_err(|source| CliError::ReadFailed {
        path: wasm_path.to_path_buf(),
        source,
    })?;
    Ok(wasm_has_placeholder_imports_bytes(&bytes))
}

fn wasm_has_placeholder_imports_bytes(bytes: &[u8]) -> bool {
    for payload in Parser::new(0).parse_all(bytes) {
        let Ok(payload) = payload else {
            continue;
        };
        let Payload::ImportSection(reader) = payload else {
            continue;
        };
        for import in reader {
            let Ok(import) = import else {
                continue;
            };
            let module = import.module;
            if module.starts_with("__wbindgen_placeholder__")
                || module.starts_with("__wbindgen_externref_xform__")
            {
                return true;
            }
        }
    }
    false
}

/// run wasm-bindgen, rename outputs to boltffi naming, patch glue, write `{module}.boltffi.json`.
pub(crate) fn run_wasm_bindgen_for_pack(
    config: &Config,
    cargo_wasm_path: &Path,
    module_name: &str,
) -> Result<()> {
    let ts_out = config.wasm_typescript_output();
    fs::create_dir_all(&ts_out).map_err(|source| CliError::CreateDirectoryFailed {
        path: ts_out.clone(),
        source,
    })?;

    let wasm_bindgen = which::which("wasm-bindgen").map_err(|_| CliError::CommandFailed {
        command:
            "wasm-bindgen not found in PATH (install wasm-bindgen-cli matching the wasm-bindgen crate version)"
                .to_string(),
        status: None,
    })?;

    let out_name = format!("{module_name}_wbg");
    let status = Command::new(wasm_bindgen)
        .arg("--keep-debug")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&ts_out)
        .arg("--out-name")
        .arg(&out_name)
        .arg("--omit-default-module-path")
        .arg("--no-typescript")
        .arg(cargo_wasm_path)
        .status()
        .map_err(|source| CliError::CommandFailed {
            command: format!("failed to spawn wasm-bindgen: {source}"),
            status: None,
        })?;

    if !status.success() {
        return Err(CliError::CommandFailed {
            command: "wasm-bindgen".to_string(),
            status: status.code(),
        });
    }

    let wbg_bg = ts_out.join(format!("{out_name}_bg.wasm"));
    let packaged_name = format!("{module_name}_bg.wasm");
    let final_wasm = ts_out.join(&packaged_name);
    fs::rename(&wbg_bg, &final_wasm).map_err(|source| CliError::WriteFailed {
        path: final_wasm.clone(),
        source,
    })?;

    let glue_js = ts_out.join(format!("{out_name}.js"));
    patch_wasm_bindgen_web_glue(&glue_js, &config.wasm_runtime_package())?;

    let marker = serde_json::json!({
        "wasm_bindgen_glue": format!("{out_name}.js"),
    });
    let marker_path = ts_out.join(format!("{module_name}.boltffi.json"));
    fs::write(
        &marker_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&marker).map_err(|e| {
                CliError::CommandFailed {
                    command: format!("serde_json: {e}"),
                    status: None,
                }
            })?
        ),
    )
    .map_err(|source| CliError::WriteFailed {
        path: marker_path,
        source,
    })?;

    Ok(())
}

/// remove stale wasm-bindgen artifacts when the wasm no longer needs them.
pub(crate) fn clear_stale_wasm_bindgen_artifacts(ts_out: &Path, module_name: &str) -> Result<()> {
    let glue = ts_out.join(format!("{module_name}_wbg.js"));
    if glue.exists() {
        fs::remove_file(&glue).map_err(|source| CliError::WriteFailed { path: glue, source })?;
    }
    let marker = ts_out.join(format!("{module_name}.boltffi.json"));
    if marker.exists() {
        fs::remove_file(&marker).map_err(|source| CliError::WriteFailed {
            path: marker,
            source,
        })?;
    }
    Ok(())
}

/// strip `import * as importN from "env"`, inject `__boltffi_takePendingEnv`, export init helpers.
fn patch_wasm_bindgen_web_glue(glue_js_path: &Path, runtime_package: &str) -> Result<()> {
    let mut s = fs::read_to_string(glue_js_path).map_err(|source| CliError::ReadFailed {
        path: glue_js_path.to_path_buf(),
        source,
    })?;

    let re = Regex::new(r#"(?m)^import \* as import\d+ from "env"\s*\n"#).map_err(|e| {
        CliError::CommandFailed {
            command: format!("regex: {e}"),
            status: None,
        }
    })?;
    // wasm-bindgen emits one `import * as importN from "env"` per env shim; count before stripping.
    let env_import_count = re.find_iter(&s).count().max(1);
    s = re.replace_all(&s, "").to_string();

    let runtime_import_line = format!(
        "import {{ __boltffi_takePendingEnv }} from \"{}\";\n",
        runtime_package
    );
    s.insert_str(0, &runtime_import_line);

    let marker_fn = "function __wbg_get_imports() {";
    let pos = s.find(marker_fn).ok_or_else(|| CliError::CommandFailed {
        command: format!(
            "wasm-bindgen glue missing `{}` in {}",
            marker_fn,
            glue_js_path.display()
        ),
        status: None,
    })?;
    let insert_at = pos + marker_fn.len();

    let mut inject = String::from("\n");
    inject.push_str("    const import1 = __boltffi_takePendingEnv();\n");
    for n in 2..=env_import_count {
        inject.push_str(&format!("    const import{n} = import1;\n"));
    }
    s.insert_str(insert_at, &inject);

    if !s.contains("export { __wbg_get_imports") {
        s.push_str(
            "\n// boltffi: wasm-bindgen helpers for BoltFFI host instantiation\nexport { __wbg_get_imports, __wbg_finalize_init };\n",
        );
    }

    fs::write(glue_js_path, s).map_err(|source| CliError::WriteFailed {
        path: glue_js_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub(crate) fn paths_differ(a: &Path, b: &Path) -> bool {
    canonicalize_or_same(a) != canonicalize_or_same(b)
}

fn canonicalize_or_same(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}
