mod npm;
mod wasm_bindgen;

use std::path::Path;
use std::process::Command;

use crate::build::{
    BuildOptions, Builder, OutputCallback, all_successful, failed_target_details, failed_targets,
};
use crate::cli::{CliError, Result};
use crate::commands::generate::{GenerateOptions, GenerateTarget, run_generate_with_output};
use crate::commands::pack::PackWasmOptions;
use crate::config::{Config, WasmNpmTarget, WasmOptimizeLevel, WasmOptimizeOnMissing, WasmProfile};
use crate::pack::PackError;
use crate::reporter::Reporter;

use super::{format_command_for_log, print_cargo_line, resolve_build_cargo_args};

use self::npm::{
    generate_wasm_loader_entrypoints, generate_wasm_package_json, generate_wasm_readme,
};

pub(crate) fn pack_wasm(
    config: &Config,
    options: PackWasmOptions,
    reporter: &Reporter,
) -> Result<()> {
    if !config.is_wasm_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.wasm.enabled = false".to_string(),
            status: None,
        });
    }
    if config.wasm_npm_generate_package_json() && config.wasm_npm_package_name().is_none() {
        return Err(CliError::CommandFailed {
            command: "targets.wasm.npm.package_name is required for pack wasm".to_string(),
            status: None,
        });
    }

    reporter.section("🌐", "Packing WASM");

    let requested_wasm_profile = if options.release {
        WasmProfile::Release
    } else {
        config.wasm_profile()
    };

    let build_cargo_args = resolve_build_cargo_args(config, &options.cargo_args);
    let build_profile = crate::build::resolve_build_profile(
        matches!(requested_wasm_profile, WasmProfile::Release),
        &build_cargo_args,
    );

    let wasm_artifact_profile = match build_profile {
        crate::build::CargoBuildProfile::Debug => WasmProfile::Debug,
        crate::build::CargoBuildProfile::Release => WasmProfile::Release,
        crate::build::CargoBuildProfile::Named(_) if config.wasm_has_artifact_path_override() => {
            requested_wasm_profile
        }
        crate::build::CargoBuildProfile::Named(profile_name) => {
            return Err(CliError::CommandFailed {
                command: format!(
                    "custom cargo profile '{}' for wasm pack requires targets.wasm.artifact_path",
                    profile_name
                ),
                status: None,
            });
        }
    };

    if !options.no_build {
        let step = reporter.step("Building WASM target");
        build_wasm_target(config, requested_wasm_profile, &build_cargo_args, &step)?;
        step.finish_success();
    }

    let wasm_artifact_path = config.wasm_artifact_path(wasm_artifact_profile);
    if !wasm_artifact_path.exists() {
        return Err(CliError::FileNotFound(wasm_artifact_path));
    }

    if config.wasm_optimize_enabled(wasm_artifact_profile) {
        let step = reporter.step("Optimizing WASM binary");
        optimize_wasm_binary(config, &wasm_artifact_path)?;
        step.finish_success();
    }

    let ts_out = config.wasm_typescript_output();
    std::fs::create_dir_all(&ts_out).map_err(|source| CliError::CreateDirectoryFailed {
        path: ts_out.clone(),
        source,
    })?;

    let module_name = config.wasm_typescript_module_name();
    let mut wasm_bindgen_ran = false;
    if wasm_bindgen::wasm_has_wasm_bindgen_placeholder_imports(&wasm_artifact_path)? {
        let step = reporter.step("Running wasm-bindgen");
        wasm_bindgen::run_wasm_bindgen_for_pack(config, &wasm_artifact_path, &module_name)?;
        wasm_bindgen_ran = true;
        step.finish_success();
    } else {
        wasm_bindgen::clear_stale_wasm_bindgen_artifacts(&ts_out, &module_name)?;
    }

    if options.regenerate || wasm_bindgen_ran {
        let step = reporter.step("Generating TypeScript bindings");
        run_generate_with_output(
            config,
            GenerateOptions {
                target: GenerateTarget::Typescript,
                output: Some(config.wasm_typescript_output()),
                experimental: false,
            },
        )?;
        step.finish_success();
    }

    let npm_output = config.wasm_npm_output();
    std::fs::create_dir_all(&npm_output).map_err(|source| CliError::CreateDirectoryFailed {
        path: npm_output.clone(),
        source,
    })?;

    let packaged_wasm_path = npm_output.join(format!("{}_bg.wasm", module_name));
    let wasm_src_for_pack = if wasm_bindgen_ran {
        ts_out.join(format!("{module_name}_bg.wasm"))
    } else {
        wasm_artifact_path.clone()
    };
    if wasm_bindgen::paths_differ(&wasm_src_for_pack, &packaged_wasm_path) {
        std::fs::copy(&wasm_src_for_pack, &packaged_wasm_path).map_err(|source| {
            CliError::CopyFailed {
                from: wasm_src_for_pack.clone(),
                to: packaged_wasm_path.clone(),
                source,
            }
        })?;
    }

    if wasm_bindgen_ran {
        let glue_src = ts_out.join(format!("{module_name}_wbg.js"));
        let glue_dst = npm_output.join(format!("{module_name}_wbg.js"));
        if wasm_bindgen::paths_differ(&glue_src, &glue_dst) {
            std::fs::copy(&glue_src, &glue_dst).map_err(|source| CliError::CopyFailed {
                from: glue_src,
                to: glue_dst,
                source,
            })?;
        }
    }

    let generated_typescript_source = config
        .wasm_typescript_output()
        .join(format!("{}.ts", module_name));
    if !generated_typescript_source.exists() {
        return Err(CliError::FileNotFound(generated_typescript_source));
    }

    let step = reporter.step("Transpiling TypeScript bindings");
    transpile_typescript_bundle(config, &generated_typescript_source, &npm_output)?;
    step.finish_success();

    let enabled_targets = config.wasm_npm_targets();
    let generated_node_typescript_source = config
        .wasm_typescript_output()
        .join(format!("{}_node.ts", module_name));
    if enabled_targets
        .iter()
        .any(|target| matches!(target, WasmNpmTarget::Nodejs))
        && generated_node_typescript_source.exists()
    {
        let step = reporter.step("Transpiling Node.js bindings");
        transpile_typescript_bundle(config, &generated_node_typescript_source, &npm_output)?;
        step.finish_success();
    }

    let step = reporter.step("Generating WASM loader entrypoints");
    generate_wasm_loader_entrypoints(&module_name, &enabled_targets, &npm_output)?;
    step.finish_success();

    if config.wasm_npm_generate_package_json() {
        let step = reporter.step("Generating package.json");
        let package_json_path =
            generate_wasm_package_json(config, &module_name, &enabled_targets, &npm_output)?;
        step.finish_success_with(&format!("{}", package_json_path.display()));
    }

    if config.wasm_npm_generate_readme() {
        let step = reporter.step("Generating README.md");
        let readme_path =
            generate_wasm_readme(config, &module_name, &enabled_targets, &npm_output)?;
        step.finish_success_with(&format!("{}", readme_path.display()));
    }

    Ok(())
}

fn build_wasm_target(
    config: &Config,
    profile: WasmProfile,
    build_cargo_args: &[String],
    step: &crate::reporter::Step,
) -> Result<()> {
    let on_output: Option<OutputCallback> = if step.is_verbose() {
        Some(Box::new(|line: &str| print_cargo_line(line)))
    } else {
        None
    };

    let build_options = BuildOptions {
        release: matches!(profile, WasmProfile::Release),
        package: Some(config.library_name().to_string()),
        cargo_args: build_cargo_args.to_vec(),
        on_output,
    };
    let builder = Builder::new(config, build_options);
    let results = builder.build_wasm_with_triple(config.wasm_triple())?;

    if all_successful(&results) {
        return Ok(());
    }

    let failed = failed_targets(&results);
    Err(PackError::BuildFailed {
        targets: failed,
        details: failed_target_details(&results),
    }
    .into())
}

fn optimize_wasm_binary(config: &Config, wasm_path: &Path) -> Result<()> {
    let optimize_level_flag = match config.wasm_optimize_level() {
        WasmOptimizeLevel::O0 => "-O0",
        WasmOptimizeLevel::O1 => "-O1",
        WasmOptimizeLevel::O2 => "-O2",
        WasmOptimizeLevel::O3 => "-O3",
        WasmOptimizeLevel::O4 => "-O4",
        WasmOptimizeLevel::Size => "-Os",
        WasmOptimizeLevel::MinSize => "-Oz",
    };

    let wasm_opt_path = match which::which("wasm-opt") {
        Ok(path) => path,
        Err(_) => {
            return match config.wasm_optimize_on_missing() {
                WasmOptimizeOnMissing::Error => Err(CliError::CommandFailed {
                    command: "wasm-opt not found in PATH".to_string(),
                    status: None,
                }),
                WasmOptimizeOnMissing::Warn => {
                    println!("warning: wasm-opt not found, skipping optimization");
                    Ok(())
                }
                WasmOptimizeOnMissing::Skip => Ok(()),
            };
        }
    };

    let optimized_path = wasm_path.with_extension("optimized.wasm");
    let mut command = Command::new(wasm_opt_path);
    command
        .arg(optimize_level_flag)
        .arg(wasm_path)
        .arg("-o")
        .arg(&optimized_path);

    if !config.wasm_optimize_strip_debug() {
        command.arg("-g");
    }

    let status = command.status().map_err(|_| CliError::CommandFailed {
        command: "wasm-opt".to_string(),
        status: None,
    })?;

    if !status.success() {
        return Err(CliError::CommandFailed {
            command: "wasm-opt".to_string(),
            status: status.code(),
        });
    }

    std::fs::rename(&optimized_path, wasm_path).map_err(|source| CliError::WriteFailed {
        path: wasm_path.to_path_buf(),
        source,
    })
}

fn transpile_typescript_bundle(
    config: &Config,
    source_file: &Path,
    output_dir: &Path,
) -> Result<()> {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "npx", "tsc"]);
        command
    } else {
        Command::new("tsc")
    };
    command
        .arg(source_file)
        .arg("--target")
        .arg("ES2020")
        .arg("--lib")
        .arg("ES2021,DOM,ESNext.Disposable")
        .arg("--module")
        .arg("ES2020")
        .arg("--moduleResolution")
        .arg("bundler")
        .arg("--declaration")
        .arg("--sourceMap")
        .arg(if config.wasm_source_map_enabled() {
            "true"
        } else {
            "false"
        })
        .arg("--skipLibCheck")
        .arg("--noEmitOnError")
        .arg("false")
        .arg("--outDir")
        .arg(output_dir);

    let command_display = format_command_for_log(&command);
    let working_directory = std::env::current_dir()
        .map(|dir| dir.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let output = command.output().map_err(|source| CliError::CommandFailed {
        command: format!(
            "failed to spawn tsc command\nworking directory: {working_directory}\ncommand: {command_display}\nerror: {source}"
        ),
        status: None,
    })?;

    let module_name = config.wasm_typescript_module_name();
    let javascript_path = output_dir.join(format!("{}.js", module_name));
    let declarations_path = output_dir.join(format!("{}.d.ts", module_name));
    let emitted_outputs_exist = javascript_path.exists() && declarations_path.exists();

    if output.status.success() || emitted_outputs_exist {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut details = Vec::new();
    if !stdout.trim().is_empty() {
        details.push(format!("stdout:\n{}", stdout.trim_end()));
    }
    if !stderr.trim().is_empty() {
        details.push(format!("stderr:\n{}", stderr.trim_end()));
    }
    let details = if details.is_empty() {
        "no stdout/stderr captured".to_string()
    } else {
        details.join("\n\n")
    };

    Err(CliError::CommandFailed {
        command: format!(
            "tsc failed while transpiling {} into {}\nworking directory: {working_directory}\ncommand: {command_display}\n{details}",
            source_file.display(),
            output_dir.display(),
        ),
        status: output.status.code(),
    })
}
