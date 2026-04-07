use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::build::{
    BuildOptions, Builder, CargoBuildProfile, OutputCallback, all_successful, failed_targets,
    resolve_build_profile, run_command_streaming,
};
use crate::commands::generate::{
    GenerateOptions, GenerateTarget, run_generate_java_with_output_from_source_dir,
    run_generate_with_output,
};
use crate::config::{
    Config, Experimental, SpmDistribution, SpmLayout, Target, WasmNpmTarget, WasmOptimizeLevel,
    WasmOptimizeOnMissing, WasmProfile,
};
use crate::desktop::DesktopToolchain;
use crate::error::{CliError, Result};
use crate::pack::{AndroidPackager, SpmPackageGenerator, XcframeworkBuilder, compute_checksum};
use crate::reporter::{Reporter, Step};
use crate::target::{BuiltLibrary, JavaHostTarget, Platform, RustTarget};

pub enum PackCommand {
    All(PackAllOptions),
    Apple(PackAppleOptions),
    Android(PackAndroidOptions),
    Wasm(PackWasmOptions),
    Java(PackJavaOptions),
}

pub struct PackAllOptions {
    pub release: bool,
    pub regenerate: bool,
    pub no_build: bool,
    pub experimental: bool,
    pub cargo_args: Vec<String>,
}

pub struct PackAppleOptions {
    pub release: bool,
    pub version: Option<String>,
    pub regenerate: bool,
    pub no_build: bool,
    pub spm_only: bool,
    pub xcframework_only: bool,
    pub layout: Option<SpmLayout>,
    pub cargo_args: Vec<String>,
}

pub struct PackAndroidOptions {
    pub release: bool,
    pub regenerate: bool,
    pub no_build: bool,
    pub cargo_args: Vec<String>,
}

pub struct PackWasmOptions {
    pub release: bool,
    pub regenerate: bool,
    pub no_build: bool,
    pub cargo_args: Vec<String>,
}

pub struct PackJavaOptions {
    pub release: bool,
    pub regenerate: bool,
    pub no_build: bool,
    pub experimental: bool,
    pub cargo_args: Vec<String>,
}

struct JvmBuildArtifacts {
    native_static_libraries: Vec<String>,
    native_link_search_paths: Vec<String>,
    static_library_filename: Option<String>,
}

struct JniLinkerArgs<'a> {
    output_lib: &'a Path,
    jni_glue: &'a Path,
    link_input: &'a Path,
    jni_dir: &'a Path,
    jni_include_directories: &'a JniIncludeDirectories,
    rustflag_linker_args: &'a [String],
    native_link_search_paths: &'a [String],
    native_static_libraries: &'a [String],
    rpath_flag: Option<&'a str>,
}

struct NativeLinkMetadata {
    native_static_libraries: Vec<String>,
    native_link_search_paths: Vec<String>,
}

#[derive(Debug, Clone)]
struct JvmCargoContext {
    host_target: JavaHostTarget,
    rust_target_triple: String,
    release: bool,
    build_profile: CargoBuildProfile,
    artifact_name: String,
    cargo_manifest_path: PathBuf,
    manifest_path: PathBuf,
    package_selector: Option<String>,
    target_directory: PathBuf,
    cargo_command_args: Vec<String>,
    toolchain_selector: Option<String>,
    crate_outputs: JvmCrateOutputs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JvmCrateOutputs {
    builds_staticlib: bool,
    builds_cdylib: bool,
}

struct JvmPackagingTarget {
    cargo_context: JvmCargoContext,
    toolchain: DesktopToolchain,
}

#[derive(Debug, Clone, Copy)]
struct JvmPackagedNativeOutput {
    host_target: JavaHostTarget,
    has_shared_library_copy: bool,
}

struct PreparedJavaPackaging {
    java_host_targets: Vec<JavaHostTarget>,
    packaging_targets: Vec<JvmPackagingTarget>,
}

#[derive(Debug)]
struct JniIncludeDirectories {
    shared: PathBuf,
    platform: PathBuf,
}

enum JvmNativeLinkInput {
    Staticlib(PathBuf),
    Cdylib(PathBuf),
}

impl JvmNativeLinkInput {
    fn path(&self) -> &Path {
        match self {
            Self::Staticlib(path) | Self::Cdylib(path) => path,
        }
    }

    fn links_staticlib(&self) -> bool {
        matches!(self, Self::Staticlib(_))
    }
}

impl JvmBuildArtifacts {
    fn static_library_filename(&self) -> Option<&str> {
        self.static_library_filename.as_deref()
    }
}

impl JvmCargoContext {
    fn artifact_directory(&self) -> PathBuf {
        self.target_directory
            .join(&self.rust_target_triple)
            .join(self.build_profile.output_directory_name())
    }
}

pub fn run_pack(config: &Config, command: PackCommand, reporter: &Reporter) -> Result<()> {
    match command {
        PackCommand::All(options) => pack_all(config, options, reporter),
        PackCommand::Apple(options) => pack_apple(config, options, reporter),
        PackCommand::Android(options) => pack_android(config, options, reporter),
        PackCommand::Wasm(options) => pack_wasm(config, options, reporter),
        PackCommand::Java(options) => pack_java(config, options, reporter),
    }
}

pub fn check_java_packaging_prereqs(
    config: &Config,
    release: bool,
    cargo_args: &[String],
) -> Result<()> {
    prepare_java_packaging(config, release, cargo_args).map(|_| ())
}

fn pack_all(config: &Config, options: PackAllOptions, reporter: &Reporter) -> Result<()> {
    ensure_java_no_build_supported(config, options.no_build, options.experimental, "pack all")?;
    let prepared_java_packaging = config
        .should_process(Target::Java, options.experimental)
        .then(|| prepare_java_packaging(config, options.release, &options.cargo_args))
        .transpose()?;

    let mut packed_any = false;

    if config.is_apple_enabled() {
        pack_apple(
            config,
            PackAppleOptions {
                release: options.release,
                version: None,
                regenerate: options.regenerate,
                no_build: options.no_build,
                spm_only: false,
                xcframework_only: false,
                layout: None,
                cargo_args: options.cargo_args.clone(),
            },
            reporter,
        )?;
        packed_any = true;
    }

    if config.is_android_enabled() {
        pack_android(
            config,
            PackAndroidOptions {
                release: options.release,
                regenerate: options.regenerate,
                no_build: options.no_build,
                cargo_args: options.cargo_args.clone(),
            },
            reporter,
        )?;
        packed_any = true;
    }

    if config.is_wasm_enabled() {
        pack_wasm(
            config,
            PackWasmOptions {
                release: options.release,
                regenerate: options.regenerate,
                no_build: options.no_build,
                cargo_args: options.cargo_args.clone(),
            },
            reporter,
        )?;
        packed_any = true;
    }

    if config.should_process(Target::Java, options.experimental) {
        pack_java_with_prepared(
            config,
            PackJavaOptions {
                release: options.release,
                regenerate: options.regenerate,
                no_build: options.no_build,
                experimental: options.experimental,
                cargo_args: options.cargo_args.clone(),
            },
            prepared_java_packaging,
            reporter,
        )?;
        packed_any = true;
    }

    if !packed_any {
        reporter.warning("no targets enabled in config");
    }

    reporter.finish();
    Ok(())
}

fn pack_apple(config: &Config, options: PackAppleOptions, reporter: &Reporter) -> Result<()> {
    if !config.is_apple_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.apple.enabled = false".to_string(),
            status: None,
        });
    }

    reporter.section("🍎", "Packing Apple");

    if !config.apple_include_macos() {
        reporter.warning("macOS excluded (targets.apple.include_macos = false)");
    }

    if options.spm_only && options.xcframework_only {
        return Err(CliError::CommandFailed {
            command: "cannot combine --spm-only and --xcframework-only".to_string(),
            status: None,
        });
    }

    let build_cargo_args = resolve_build_cargo_args(config, &options.cargo_args);
    let build_profile = resolve_build_profile(options.release, &build_cargo_args);
    let apple_targets = config.apple_targets();

    if !options.no_build {
        let step = reporter.step("Building Apple targets");
        build_apple_targets(
            config,
            &apple_targets,
            options.release,
            &build_cargo_args,
            &step,
        )?;
        step.finish_success();
    }

    let layout = options.layout.unwrap_or_else(|| config.apple_spm_layout());
    let package_root = config.apple_spm_output();

    if options.regenerate {
        let step = reporter.step("Generating Apple bindings");
        generate_apple_bindings(config, layout, &package_root)?;
        step.finish_success();
    }

    let libraries = discover_built_libraries_for_targets(
        &config.crate_artifact_name(),
        build_profile.output_directory_name(),
        &apple_targets,
    )?;
    let apple_libraries: Vec<_> = libraries
        .into_iter()
        .filter(|lib| lib.target.platform().is_apple())
        .collect();

    let missing_targets = missing_built_libraries(&apple_targets, &apple_libraries);
    if !missing_targets.is_empty() {
        return Err(CliError::MissingBuiltLibraries {
            platform: "Apple".to_string(),
            targets: missing_targets,
        });
    }

    let headers_dir = config.apple_header_output();
    if !headers_dir.exists() {
        return Err(CliError::FileNotFound(headers_dir));
    }

    let should_build_xcframework = !options.spm_only;
    let should_generate_spm = !options.xcframework_only;

    let xcframework_output = if should_build_xcframework {
        let step = reporter.step("Creating xcframework");
        let output = XcframeworkBuilder::new(config, apple_libraries.clone(), headers_dir.clone())
            .build_with_zip()?;
        step.finish_success();
        Some(output)
    } else {
        None
    };

    if should_generate_spm {
        let (checksum, version) = match config.apple_spm_distribution() {
            SpmDistribution::Local => (None, None),
            SpmDistribution::Remote => {
                let checksum = xcframework_output
                    .as_ref()
                    .and_then(|o| o.checksum.clone())
                    .map(Ok)
                    .unwrap_or_else(|| {
                        let step = reporter.step("Computing checksum");
                        let result = existing_xcframework_checksum(config);
                        step.finish_success();
                        result
                    })?;
                let version = options
                    .version
                    .or_else(detect_version)
                    .unwrap_or_else(|| "0.1.0".to_string());
                (Some(checksum), Some(version))
            }
        };

        if config.apple_spm_skip_package_swift() {
            reporter.warning("Skipping Package.swift (skip_package_swift = true)");
        } else {
            let generator = match config.apple_spm_distribution() {
                SpmDistribution::Local => SpmPackageGenerator::new_local(config, layout),
                SpmDistribution::Remote => {
                    let checksum = checksum.ok_or_else(|| CliError::CommandFailed {
                        command: "remote SPM requires checksum".to_string(),
                        status: None,
                    })?;
                    let version = version.ok_or_else(|| CliError::CommandFailed {
                        command: "remote SPM requires version".to_string(),
                        status: None,
                    })?;
                    SpmPackageGenerator::new_remote(config, checksum, version, layout)
                }
            };

            let step = reporter.step("Generating Package.swift");
            let package_path = generator.generate()?;
            step.finish_success_with(&format!("{}", package_path.display()));
        }
    }

    Ok(())
}

fn pack_wasm(config: &Config, options: PackWasmOptions, reporter: &Reporter) -> Result<()> {
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
    let build_profile = resolve_build_profile(
        matches!(requested_wasm_profile, WasmProfile::Release),
        &build_cargo_args,
    );

    let wasm_artifact_profile = match build_profile {
        CargoBuildProfile::Debug => WasmProfile::Debug,
        CargoBuildProfile::Release => WasmProfile::Release,
        CargoBuildProfile::Named(_) if config.wasm_has_artifact_path_override() => {
            requested_wasm_profile
        }
        CargoBuildProfile::Named(profile_name) => {
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

    if options.regenerate {
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

    let module_name = config.wasm_typescript_module_name();
    let packaged_wasm_path = npm_output.join(format!("{}_bg.wasm", module_name));
    std::fs::copy(&wasm_artifact_path, &packaged_wasm_path).map_err(|source| {
        CliError::CopyFailed {
            from: wasm_artifact_path.clone(),
            to: packaged_wasm_path.clone(),
            source,
        }
    })?;

    let generated_typescript_source = config
        .wasm_typescript_output()
        .join(format!("{}.ts", module_name));
    if !generated_typescript_source.exists() {
        return Err(CliError::FileNotFound(generated_typescript_source));
    }

    let step = reporter.step("Transpiling TypeScript bindings");
    transpile_typescript_bundle(config, &generated_typescript_source, &npm_output)?;
    step.finish_success();

    let generated_node_typescript_source = config
        .wasm_typescript_output()
        .join(format!("{}_node.ts", module_name));
    if generated_node_typescript_source.exists() {
        let step = reporter.step("Transpiling Node.js bindings");
        transpile_typescript_bundle(config, &generated_node_typescript_source, &npm_output)?;
        step.finish_success();
    }

    let enabled_targets = config.wasm_npm_targets();
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

fn pack_android(config: &Config, options: PackAndroidOptions, reporter: &Reporter) -> Result<()> {
    if !config.is_android_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.android.enabled = false".to_string(),
            status: None,
        });
    }

    reporter.section("🤖", "Packing Android");

    let build_cargo_args = resolve_build_cargo_args(config, &options.cargo_args);
    let build_profile = resolve_build_profile(options.release, &build_cargo_args);
    let android_targets = config.android_targets();

    if !options.no_build {
        let step = reporter.step("Building Android targets");
        build_android_targets(
            config,
            &android_targets,
            options.release,
            &build_cargo_args,
            &step,
        )?;
        step.finish_success();
    }

    if options.regenerate {
        let step = reporter.step("Generating Kotlin bindings");
        run_generate_with_output(
            config,
            GenerateOptions {
                target: GenerateTarget::Kotlin,
                output: Some(config.android_kotlin_output()),
                experimental: false,
            },
        )?;
        step.finish_success();

        let step = reporter.step("Generating C header");
        run_generate_with_output(
            config,
            GenerateOptions {
                target: GenerateTarget::Header,
                output: Some(config.android_header_output()),
                experimental: false,
            },
        )?;
        step.finish_success();
    }

    let libraries = discover_built_libraries_for_targets(
        &config.crate_artifact_name(),
        build_profile.output_directory_name(),
        &android_targets,
    )?;
    let android_libraries: Vec<_> = libraries
        .into_iter()
        .filter(|lib| lib.target.platform() == Platform::Android)
        .collect();

    let missing_targets = missing_built_libraries(&android_targets, &android_libraries);
    if !missing_targets.is_empty() {
        return Err(CliError::MissingBuiltLibraries {
            platform: "Android".to_string(),
            targets: missing_targets,
        });
    }

    let packager = AndroidPackager::new(config, android_libraries, build_profile.is_release_like());
    let step = reporter.step("Packaging jniLibs");
    packager.package()?;
    step.finish_success();

    Ok(())
}

fn pack_java(config: &Config, options: PackJavaOptions, reporter: &Reporter) -> Result<()> {
    pack_java_with_prepared(config, options, None, reporter)
}

fn pack_java_with_prepared(
    config: &Config,
    options: PackJavaOptions,
    prepared: Option<PreparedJavaPackaging>,
    reporter: &Reporter,
) -> Result<()> {
    if !config.is_java_jvm_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.java.jvm.enabled = false".to_string(),
            status: None,
        });
    }

    reporter.section("☕", "Packing Java");

    ensure_java_pack_experimental_supported(config, options.experimental)?;
    ensure_java_no_build_supported(config, options.no_build, options.experimental, "pack java")?;

    let PreparedJavaPackaging {
        java_host_targets,
        packaging_targets,
    } = if let Some(prepared) = prepared {
        prepared
    } else {
        let step = reporter.step("Validating JVM toolchains");
        let prepared = prepare_java_packaging(config, options.release, &options.cargo_args)?;
        step.finish_success();
        prepared
    };

    if options.regenerate {
        let source_directory = selected_jvm_package_source_directory(&packaging_targets)?;
        let artifact_name = selected_jvm_package_artifact_name(&packaging_targets)?;
        let step = reporter.step("Generating C header");
        generate_java_header(config, &source_directory, artifact_name)?;
        step.finish_success();

        let step = reporter.step("Generating Java bindings");
        run_generate_java_with_output_from_source_dir(
            config,
            Some(config.java_jvm_output()),
            options.experimental,
            &source_directory,
            artifact_name,
        )?;
        step.finish_success();
    }

    let mut packaged_outputs = Vec::with_capacity(packaging_targets.len());
    for packaging_target in &packaging_targets {
        let host_target = packaging_target.cargo_context.host_target;
        let step = reporter.step(&format!(
            "Building Rust library for {}",
            host_target.canonical_name()
        ));
        let build_artifacts = build_jvm_native_library(packaging_target, options.release, &step)?;
        step.finish_success();

        let step = reporter.step(&format!(
            "Compiling JNI library for {}",
            host_target.canonical_name()
        ));
        packaged_outputs.push(compile_jni_library(
            config,
            packaging_target,
            &build_artifacts,
        )?);
        step.finish_success();
    }

    let artifact_name = selected_jvm_package_artifact_name(&packaging_targets)?;
    remove_stale_requested_jvm_shared_library_copies_after_success(
        &config.java_jvm_output(),
        &packaged_outputs,
        artifact_name,
    )?;
    remove_stale_structured_jvm_outputs(
        &config.java_jvm_output().join("native"),
        &java_host_targets,
    )?;
    remove_stale_flat_jvm_outputs_if_current_host_unrequested(
        &config.java_jvm_output(),
        JavaHostTarget::current(),
        &java_host_targets,
        artifact_name,
    )?;

    reporter.finish();
    Ok(())
}

fn prepare_java_packaging(
    config: &Config,
    release: bool,
    cargo_args: &[String],
) -> Result<PreparedJavaPackaging> {
    let build_cargo_args = resolve_build_cargo_args(config, cargo_args);
    ensure_java_pack_cargo_args_supported(&build_cargo_args)?;
    let build_profile = resolve_build_profile(release, &build_cargo_args);
    let java_host_targets = resolve_java_host_targets_for_packaging(config)?;
    let packaging_targets = resolve_jvm_packaging_targets(
        config,
        &build_cargo_args,
        release,
        build_profile,
        &java_host_targets,
    )?;

    Ok(PreparedJavaPackaging {
        java_host_targets,
        packaging_targets,
    })
}

fn ensure_java_no_build_supported(
    config: &Config,
    no_build: bool,
    experimental: bool,
    command_name: &str,
) -> Result<()> {
    if no_build && config.should_process(Target::Java, experimental) {
        return Err(CliError::CommandFailed {
            command: format!(
                "{command_name} --no-build is unsupported in Phase 4 when JVM packaging is enabled; rerun without --no-build"
            ),
            status: None,
        });
    }

    Ok(())
}

fn ensure_java_pack_experimental_supported(config: &Config, experimental: bool) -> Result<()> {
    if !Experimental::is_target_experimental(Target::Java) {
        return Ok(());
    }

    if experimental
        || config
            .experimental
            .iter()
            .any(|entry| entry == Target::Java.name())
    {
        return Ok(());
    }

    Err(CliError::CommandFailed {
        command: "java is experimental, use --experimental flag or add \"java\" to [experimental]"
            .to_string(),
        status: None,
    })
}

fn ensure_java_pack_cargo_args_supported(cargo_args: &[String]) -> Result<()> {
    if let Some(target_selector) = current_cargo_target_selector(cargo_args) {
        return Err(CliError::CommandFailed {
            command: format!(
                "pack java resolves desktop targets from targets.java.jvm.host_targets; remove cargo --target '{}'",
                target_selector
            ),
            status: None,
        });
    }

    Ok(())
}

fn generate_java_header(config: &Config, source_directory: &Path, crate_name: &str) -> Result<()> {
    use boltffi_bindgen::{CHeaderLowerer, ir, scan_crate_with_pointer_width};

    let output_dir = config.java_jvm_output().join("jni");
    let output_path = output_dir.join(format!("{crate_name}.h"));

    std::fs::create_dir_all(&output_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: output_dir.clone(),
        source,
    })?;
    let host_pointer_width_bits = match usize::BITS {
        32 => Some(32),
        64 => Some(64),
        _ => None,
    };
    let mut module =
        scan_crate_with_pointer_width(source_directory, crate_name, host_pointer_width_bits)
            .map_err(|error| CliError::CommandFailed {
                command: format!("scan_crate: {}", error),
                status: None,
            })?;

    let contract = ir::build_contract(&mut module);
    let abi = ir::Lowerer::new(&contract).to_abi_contract();
    let header_code = CHeaderLowerer::new(&contract, &abi).generate();
    std::fs::write(&output_path, header_code).map_err(|source| CliError::WriteFailed {
        path: output_path,
        source,
    })?;

    Ok(())
}

fn selected_jvm_package_source_directory(
    packaging_targets: &[JvmPackagingTarget],
) -> Result<PathBuf> {
    packaging_targets
        .first()
        .and_then(|target| target.cargo_context.manifest_path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| CliError::CommandFailed {
            command: "could not resolve selected Cargo package source directory for JVM generation"
                .to_string(),
            status: None,
        })
}

fn selected_jvm_package_artifact_name(packaging_targets: &[JvmPackagingTarget]) -> Result<&str> {
    packaging_targets
        .first()
        .map(|target| target.cargo_context.artifact_name.as_str())
        .ok_or_else(|| CliError::CommandFailed {
            command: "could not resolve selected Cargo package artifact name for JVM generation"
                .to_string(),
            status: None,
        })
}

fn compile_jni_library(
    config: &Config,
    packaging_target: &JvmPackagingTarget,
    build_artifacts: &JvmBuildArtifacts,
) -> Result<JvmPackagedNativeOutput> {
    let cargo_context = &packaging_target.cargo_context;
    let host_target = cargo_context.host_target;
    let java_output = config.java_jvm_output();
    let jni_dir = java_output.join("jni");
    let jni_glue = jni_dir.join("jni_glue.c");
    let header = jni_dir.join(format!("{}.h", cargo_context.artifact_name));

    if !jni_glue.exists() {
        return Err(CliError::FileNotFound(jni_glue));
    }
    if !header.exists() {
        return Err(CliError::FileNotFound(header));
    }

    let artifact_name = &cargo_context.artifact_name;
    let link_input = resolve_jvm_native_link_input(
        &cargo_context.artifact_directory(),
        host_target,
        artifact_name,
        cargo_context.crate_outputs,
        build_artifacts.static_library_filename(),
    )?;
    let compatibility_shared_library = bundled_jvm_shared_library_path(
        &link_input,
        &cargo_context.artifact_directory(),
        host_target,
        artifact_name,
        cargo_context.crate_outputs,
    );

    let host_native_output = java_output
        .join("native")
        .join(host_target.canonical_name());
    std::fs::create_dir_all(&host_native_output).map_err(|source| {
        CliError::CreateDirectoryFailed {
            path: host_native_output.clone(),
            source,
        }
    })?;

    let output_lib = host_native_output.join(host_target.jni_library_filename(artifact_name));
    let jni_include_directories = resolve_jni_include_directories(cargo_context)?;
    let has_shared_library_copy = compatibility_shared_library.is_some();

    let mut cmd = packaging_target.toolchain.linker_command();
    let jni_linker_args = if packaging_target.toolchain.uses_msvc_compiler() {
        clang_cl_jni_linker_args(&JniLinkerArgs {
            output_lib: &output_lib,
            jni_glue: &jni_glue,
            link_input: link_input.path(),
            jni_dir: &jni_dir,
            jni_include_directories: &jni_include_directories,
            rustflag_linker_args: packaging_target.toolchain.jni_rustflag_linker_args(),
            native_link_search_paths: &build_artifacts.native_link_search_paths,
            native_static_libraries: &build_artifacts.native_static_libraries,
            rpath_flag: None,
        })?
    } else {
        clang_style_jni_linker_args(&JniLinkerArgs {
            output_lib: &output_lib,
            jni_glue: &jni_glue,
            link_input: link_input.path(),
            jni_dir: &jni_dir,
            jni_include_directories: &jni_include_directories,
            rustflag_linker_args: packaging_target.toolchain.jni_rustflag_linker_args(),
            native_link_search_paths: &build_artifacts.native_link_search_paths,
            native_static_libraries: &build_artifacts.native_static_libraries,
            rpath_flag: host_target.rpath_flag(),
        })
    };
    cmd.args(jni_linker_args);

    let status = cmd.status().map_err(|e| CliError::CommandFailed {
        command: format!("desktop linker: {}", e),
        status: None,
    })?;

    if !status.success() {
        return Err(CliError::CommandFailed {
            command: format!(
                "desktop linker failed to compile JNI library for '{}'",
                host_target.canonical_name()
            ),
            status: status.code(),
        });
    }

    let current_host = JavaHostTarget::current();
    if current_host == Some(host_target) {
        let compatibility_jni_copy =
            java_output.join(host_target.jni_library_filename(artifact_name));
        std::fs::copy(&output_lib, &compatibility_jni_copy).map_err(|source| {
            CliError::CopyFailed {
                from: output_lib.clone(),
                to: compatibility_jni_copy,
                source,
            }
        })?;
    }

    if let Some(shared_library) = compatibility_shared_library.as_deref() {
        let shared_library_name = shared_library
            .file_name()
            .expect("shared library path should have a file name");
        let structured_copy = host_native_output.join(shared_library_name);
        std::fs::copy(shared_library, &structured_copy).map_err(|source| CliError::CopyFailed {
            from: shared_library.to_path_buf(),
            to: structured_copy,
            source,
        })?;

        if current_host == Some(host_target) {
            let flat_copy = java_output.join(shared_library_name);
            std::fs::copy(shared_library, &flat_copy).map_err(|source| CliError::CopyFailed {
                from: shared_library.to_path_buf(),
                to: flat_copy,
                source,
            })?;
        }
    }

    Ok(JvmPackagedNativeOutput {
        host_target,
        has_shared_library_copy,
    })
}

fn build_jvm_native_library(
    packaging_target: &JvmPackagingTarget,
    release: bool,
    step: &Step,
) -> Result<JvmBuildArtifacts> {
    let cargo_context = &packaging_target.cargo_context;
    let native_static_libraries = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured_static_libraries = Arc::clone(&native_static_libraries);
    let verbose = step.is_verbose();
    let on_output: Option<OutputCallback> = Some(Box::new(move |line: &str| {
        if verbose {
            print_cargo_line(line);
        }

        if let Some(flags) = parse_native_static_libraries(line) {
            let mut libraries = captured_static_libraries
                .lock()
                .expect("native static libraries lock poisoned");
            *libraries = flags;
        }
    }));

    let crate_dir = std::env::current_dir().map_err(|source| CliError::CommandFailed {
        command: format!("current_dir: {source}"),
        status: None,
    })?;
    let mut command = Command::new("cargo");
    command.current_dir(crate_dir);

    if let Some(toolchain_selector) = cargo_context.toolchain_selector.as_deref() {
        command.arg(toolchain_selector);
    }

    command
        .arg("build")
        .arg("--target")
        .arg(&cargo_context.rust_target_triple);
    apply_jvm_cargo_package_selection(&mut command, cargo_context);

    if release {
        command.arg("--release");
    }

    command.args(&cargo_context.cargo_command_args);
    packaging_target
        .toolchain
        .configure_cargo_build(&mut command);

    if !run_command_streaming(&mut command, on_output.as_ref()) {
        return Err(CliError::BuildFailed {
            targets: vec![cargo_context.host_target.canonical_name().to_string()],
        });
    }

    let native_static_libraries = native_static_libraries
        .lock()
        .expect("native static libraries lock poisoned")
        .clone();
    let mut native_link_search_paths = Vec::new();

    let native_static_libraries = if native_static_libraries.is_empty() {
        let static_library_filename = if cargo_context.crate_outputs.builds_staticlib {
            resolve_static_library_filename(cargo_context)?
        } else {
            None
        };
        let staticlib_path = static_library_filename
            .as_ref()
            .map(|filename| cargo_context.artifact_directory().join(filename));

        if cargo_context.crate_outputs.builds_staticlib
            && staticlib_path
                .as_ref()
                .is_some_and(|staticlib_path| staticlib_path.exists())
        {
            let link_metadata = query_native_link_metadata(packaging_target, release)?;
            native_link_search_paths = link_metadata.native_link_search_paths;
            link_metadata.native_static_libraries
        } else {
            native_static_libraries
        }
    } else {
        let static_library_filename = if cargo_context.crate_outputs.builds_staticlib {
            resolve_static_library_filename(cargo_context)?
        } else {
            None
        };
        let staticlib_path = static_library_filename
            .as_ref()
            .map(|filename| cargo_context.artifact_directory().join(filename));

        if cargo_context.crate_outputs.builds_staticlib
            && staticlib_path
                .as_ref()
                .is_some_and(|staticlib_path| staticlib_path.exists())
        {
            native_link_search_paths =
                query_native_link_metadata(packaging_target, release)?.native_link_search_paths;
        }

        native_static_libraries
    };

    let static_library_filename = if cargo_context.crate_outputs.builds_staticlib {
        resolve_static_library_filename(cargo_context)?
    } else {
        None
    };

    Ok(JvmBuildArtifacts {
        native_static_libraries,
        native_link_search_paths,
        static_library_filename,
    })
}

fn resolve_java_host_targets_for_packaging(config: &Config) -> Result<Vec<JavaHostTarget>> {
    config
        .java_jvm_host_targets()
        .map_err(|message| CliError::CommandFailed {
            command: message,
            status: None,
        })
}

fn resolve_jvm_packaging_targets(
    config: &Config,
    build_cargo_args: &[String],
    release: bool,
    build_profile: CargoBuildProfile,
    host_targets: &[JavaHostTarget],
) -> Result<Vec<JvmPackagingTarget>> {
    let current_host = JavaHostTarget::current().ok_or_else(|| CliError::CommandFailed {
        command:
            "JVM packaging is only supported on darwin-arm64, darwin-x86_64, linux-x86_64, linux-aarch64, and windows-x86_64 hosts".to_string(),
        status: None,
    })?;
    let metadata = cargo_metadata_with_args(build_cargo_args)?;
    let cargo_manifest_path = current_manifest_path_with_args(build_cargo_args)?;
    let package_selector =
        effective_cargo_package_selector(config, build_cargo_args, &metadata, &cargo_manifest_path);
    let package =
        find_cargo_metadata_package(&metadata, &cargo_manifest_path, package_selector.as_deref())?;
    let artifact_name = resolve_jvm_package_artifact_name(
        package,
        &config.crate_artifact_name(),
        &cargo_manifest_path,
    )?
    .to_string();
    let probe_cargo_args = strip_cargo_manifest_path_selectors(&strip_cargo_target_selectors(
        &strip_cargo_package_selectors(build_cargo_args),
    ));
    let (toolchain_selector, cargo_command_args) = split_toolchain_selector(&probe_cargo_args);
    let crate_outputs = parse_jvm_crate_outputs(
        &metadata,
        &artifact_name,
        &cargo_manifest_path,
        package_selector.as_deref(),
    )?;

    host_targets
        .iter()
        .copied()
        .map(|host_target| {
            let toolchain = DesktopToolchain::discover(
                toolchain_selector.as_deref(),
                &cargo_command_args,
                host_target,
                current_host,
            )?;
            let cargo_context = JvmCargoContext {
                host_target,
                rust_target_triple: toolchain.rust_target_triple().to_string(),
                release,
                build_profile: build_profile.clone(),
                artifact_name: artifact_name.clone(),
                cargo_manifest_path: cargo_manifest_path.clone(),
                manifest_path: package.manifest_path.clone(),
                package_selector: package_selector.clone(),
                target_directory: metadata.target_directory.clone(),
                cargo_command_args: cargo_command_args.clone(),
                toolchain_selector: toolchain_selector.clone(),
                crate_outputs,
            };
            let _ = resolve_jni_include_directories(&cargo_context)?;
            Ok(JvmPackagingTarget {
                cargo_context,
                toolchain,
            })
        })
        .collect()
}

fn resolve_jvm_native_link_input(
    artifact_directory: &Path,
    host_target: JavaHostTarget,
    artifact_name: &str,
    crate_outputs: JvmCrateOutputs,
    static_library_filename: Option<&str>,
) -> Result<JvmNativeLinkInput> {
    let staticlib_path = static_library_filename.map(|filename| artifact_directory.join(filename));
    if crate_outputs.builds_staticlib
        && staticlib_path
            .as_ref()
            .is_some_and(|staticlib_path| staticlib_path.exists())
    {
        return Ok(JvmNativeLinkInput::Staticlib(
            staticlib_path.expect("checked staticlib path existence"),
        ));
    }

    let cdylib_path = artifact_directory.join(host_target.shared_library_filename(artifact_name));
    if crate_outputs.builds_cdylib && cdylib_path.exists() {
        return Ok(JvmNativeLinkInput::Cdylib(cdylib_path));
    }

    if crate_outputs.builds_staticlib {
        return Err(CliError::FileNotFound(staticlib_path.unwrap_or_else(
            || artifact_directory.join(host_target.static_library_filename(artifact_name)),
        )));
    }

    if crate_outputs.builds_cdylib {
        return Err(CliError::FileNotFound(cdylib_path));
    }

    Err(CliError::CommandFailed {
        command:
            "the current library target must enable either staticlib or cdylib for JVM packaging"
                .to_string(),
        status: None,
    })
}

fn existing_jvm_shared_library_path(
    artifact_directory: &Path,
    host_target: JavaHostTarget,
    artifact_name: &str,
    crate_outputs: JvmCrateOutputs,
) -> Option<PathBuf> {
    if !crate_outputs.builds_cdylib {
        return None;
    }

    let shared_library_path =
        artifact_directory.join(host_target.shared_library_filename(artifact_name));
    shared_library_path.exists().then_some(shared_library_path)
}

fn bundled_jvm_shared_library_path(
    link_input: &JvmNativeLinkInput,
    artifact_directory: &Path,
    host_target: JavaHostTarget,
    artifact_name: &str,
    crate_outputs: JvmCrateOutputs,
) -> Option<PathBuf> {
    if link_input.links_staticlib() {
        return None;
    }

    existing_jvm_shared_library_path(
        artifact_directory,
        host_target,
        artifact_name,
        crate_outputs,
    )
}

fn parse_native_static_libraries(line: &str) -> Option<Vec<String>> {
    let (_, flags) = line.split_once("native-static-libs:")?;
    let parsed: Vec<String> = flags
        .split_whitespace()
        .map(str::to_string)
        .filter(|flag| !flag.is_empty())
        .collect();

    (!parsed.is_empty()).then_some(parsed)
}

fn resolve_jni_include_directories(
    cargo_context: &JvmCargoContext,
) -> Result<JniIncludeDirectories> {
    let java_home_override_env =
        target_specific_java_home_env_key(&cargo_context.rust_target_triple);
    let include_override_env =
        target_specific_java_include_env_key(&cargo_context.rust_target_triple);
    resolve_jni_include_directories_with_overrides(
        cargo_context,
        std::env::var_os("JAVA_HOME").map(PathBuf::from),
        std::env::var_os(&java_home_override_env).map(PathBuf::from),
        std::env::var_os(&include_override_env).map(PathBuf::from),
    )
}

fn resolve_jni_include_directories_with_overrides(
    cargo_context: &JvmCargoContext,
    default_java_home: Option<PathBuf>,
    target_java_home_override: Option<PathBuf>,
    target_include_override: Option<PathBuf>,
) -> Result<JniIncludeDirectories> {
    let java_home_override_env =
        target_specific_java_home_env_key(&cargo_context.rust_target_triple);
    let include_override_env =
        target_specific_java_include_env_key(&cargo_context.rust_target_triple);
    let platform_include = target_include_override.clone().unwrap_or_else(|| {
        target_java_home_override
            .clone()
            .or(default_java_home.clone())
            .map(|java_home| {
                java_home
                    .join("include")
                    .join(cargo_context.host_target.jni_platform())
            })
            .unwrap_or_default()
    });

    let shared_include = target_include_override
        .as_ref()
        .and_then(|platform_include| platform_include.parent().map(Path::to_path_buf))
        .or_else(|| {
            target_java_home_override
                .or(default_java_home)
                .map(|java_home| java_home.join("include"))
        })
        .or_else(|| platform_include.parent().map(Path::to_path_buf))
        .ok_or_else(|| CliError::CommandFailed {
            command: format!(
                "JAVA_HOME not set; for cross-host JVM packaging you can also set {} or {}",
                java_home_override_env, include_override_env
            ),
            status: None,
        })?;

    if !shared_include.exists() {
        return Err(CliError::FileNotFound(shared_include));
    }

    let shared_header = shared_include.join("jni.h");
    if !shared_header.exists() {
        return Err(CliError::FileNotFound(shared_header));
    }

    if !platform_include.exists() {
        return Err(CliError::CommandFailed {
            command: format!(
                "missing JNI platform headers for '{}' at '{}'; set {} to a directory containing jni_md.h or set {} to a target-specific JDK home",
                cargo_context.host_target.canonical_name(),
                platform_include.display(),
                include_override_env,
                java_home_override_env
            ),
            status: None,
        });
    }

    let platform_header = platform_include.join("jni_md.h");
    if !platform_header.exists() {
        return Err(CliError::FileNotFound(platform_header));
    }

    Ok(JniIncludeDirectories {
        shared: shared_include,
        platform: platform_include,
    })
}

fn target_specific_java_home_env_key(rust_target_triple: &str) -> String {
    format!(
        "BOLTFFI_JAVA_HOME_{}",
        rust_target_triple.replace('-', "_").to_uppercase()
    )
}

fn target_specific_java_include_env_key(rust_target_triple: &str) -> String {
    format!(
        "BOLTFFI_JAVA_INCLUDE_{}",
        rust_target_triple.replace('-', "_").to_uppercase()
    )
}

fn query_native_link_metadata(
    packaging_target: &JvmPackagingTarget,
    release: bool,
) -> Result<NativeLinkMetadata> {
    let cargo_context = &packaging_target.cargo_context;
    let crate_dir = std::env::current_dir().map_err(|source| CliError::CommandFailed {
        command: format!("current_dir: {source}"),
        status: None,
    })?;

    let mut command = Command::new("cargo");
    command.current_dir(crate_dir);

    if let Some(toolchain_selector) = cargo_context.toolchain_selector.as_deref() {
        command.arg(toolchain_selector);
    }

    command
        .arg("rustc")
        .arg("--target")
        .arg(&cargo_context.rust_target_triple);
    apply_jvm_cargo_package_selection(&mut command, cargo_context);

    if release {
        command.arg("--release");
    }

    command
        .args(&cargo_context.cargo_command_args)
        .arg("--message-format=json-render-diagnostics")
        .arg("--lib")
        .arg("--")
        .arg("--print=native-static-libs");
    packaging_target
        .toolchain
        .configure_cargo_build(&mut command);

    let output = command.output().map_err(|source| CliError::CommandFailed {
        command: format!("cargo rustc --print=native-static-libs: {source}"),
        status: None,
    })?;

    if !output.status.success() {
        return Err(CliError::CommandFailed {
            command: "cargo rustc --print=native-static-libs".to_string(),
            status: output.status.code(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    let native_link_search_paths = extract_link_search_paths(&stdout);
    let native_static_libraries =
        extract_native_static_libraries(&combined).ok_or_else(|| CliError::CommandFailed {
            command: "cargo rustc --print=native-static-libs did not emit link metadata"
                .to_string(),
            status: None,
        })?;

    Ok(NativeLinkMetadata {
        native_static_libraries,
        native_link_search_paths,
    })
}

fn resolve_static_library_filename(cargo_context: &JvmCargoContext) -> Result<Option<String>> {
    let artifact_name = &cargo_context.artifact_name;

    if cargo_context.host_target != JavaHostTarget::WindowsX86_64 {
        return Ok(Some(
            cargo_context
                .host_target
                .static_library_filename(artifact_name),
        ));
    }

    let filenames = query_library_filenames(cargo_context)?;
    select_windows_static_library_filename(artifact_name, &filenames)
        .map(Some)
        .ok_or_else(|| CliError::CommandFailed {
            command: format!(
                "cargo rustc --print=file-names did not report a Windows static library for '{}'",
                artifact_name
            ),
            status: None,
        })
}

fn query_library_filenames(cargo_context: &JvmCargoContext) -> Result<Vec<String>> {
    let crate_dir = std::env::current_dir().map_err(|source| CliError::CommandFailed {
        command: format!("current_dir: {source}"),
        status: None,
    })?;

    let mut command = Command::new("cargo");
    command.current_dir(crate_dir);

    if let Some(toolchain_selector) = cargo_context.toolchain_selector.as_deref() {
        command.arg(toolchain_selector);
    }

    command
        .arg("rustc")
        .arg("--target")
        .arg(&cargo_context.rust_target_triple);
    apply_jvm_cargo_package_selection(&mut command, cargo_context);

    if cargo_context.release {
        command.arg("--release");
    }

    command
        .args(&cargo_context.cargo_command_args)
        .arg("--lib")
        .arg("--")
        .arg("--print=file-names");

    let output = command.output().map_err(|source| CliError::CommandFailed {
        command: format!("cargo rustc --print=file-names: {source}"),
        status: None,
    })?;

    if !output.status.success() {
        return Err(CliError::CommandFailed {
            command: "cargo rustc --print=file-names".to_string(),
            status: output.status.code(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    let filenames = extract_library_filenames(&combined);

    if filenames.is_empty() {
        return Err(CliError::CommandFailed {
            command: "cargo rustc --print=file-names did not emit any library filenames"
                .to_string(),
            status: None,
        });
    }

    Ok(filenames)
}

fn extract_library_filenames(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.contains(' ')
                && [".a", ".lib", ".dylib", ".so", ".rlib", ".dll"]
                    .iter()
                    .any(|extension| line.ends_with(extension))
        })
        .map(str::to_string)
        .collect()
}

fn select_windows_static_library_filename(
    artifact_name: &str,
    filenames: &[String],
) -> Option<String> {
    let msvc_name = format!("{artifact_name}.lib");
    let gnu_name = format!("lib{artifact_name}.a");

    filenames
        .iter()
        .find(|filename| *filename == &msvc_name || *filename == &gnu_name)
        .cloned()
}

fn extract_native_static_libraries(output: &str) -> Option<Vec<String>> {
    output
        .lines()
        .filter_map(parse_native_static_libraries)
        .next_back()
}

fn extract_link_search_paths(output: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct BuildScriptExecutedMessage {
        reason: String,
        #[serde(default)]
        linked_paths: Vec<String>,
    }

    let mut linked_paths = Vec::new();

    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
    {
        let Ok(message) = serde_json::from_str::<BuildScriptExecutedMessage>(line) else {
            continue;
        };

        if message.reason != "build-script-executed" {
            continue;
        }

        for linked_path in message.linked_paths {
            if !linked_paths.contains(&linked_path) {
                linked_paths.push(linked_path);
            }
        }
    }

    linked_paths
}

fn link_search_path_flags(link_search_paths: &[String]) -> Vec<String> {
    let mut flags = Vec::new();

    for linked_path in link_search_paths {
        let flag = if let Some(path) = linked_path.strip_prefix("framework=") {
            format!("-F{path}")
        } else if let Some(path) = linked_path.strip_prefix("native=") {
            format!("-L{path}")
        } else if let Some(path) = linked_path.strip_prefix("dependency=") {
            format!("-L{path}")
        } else if let Some(path) = linked_path.strip_prefix("all=") {
            format!("-L{path}")
        } else if let Some(path) = linked_path.strip_prefix("crate=") {
            format!("-L{path}")
        } else {
            format!("-L{linked_path}")
        };

        if !flags.contains(&flag) {
            flags.push(flag);
        }
    }

    flags
}

fn clang_style_jni_linker_args(args_in: &JniLinkerArgs<'_>) -> Vec<String> {
    let mut args = vec![
        "-shared".to_string(),
        "-fPIC".to_string(),
        "-o".to_string(),
        args_in.output_lib.display().to_string(),
        args_in.jni_glue.display().to_string(),
        args_in.link_input.display().to_string(),
        format!("-I{}", args_in.jni_dir.display()),
        format!("-I{}", args_in.jni_include_directories.shared.display()),
        format!("-I{}", args_in.jni_include_directories.platform.display()),
    ];
    args.extend(args_in.rustflag_linker_args.iter().cloned());
    args.extend(link_search_path_flags(args_in.native_link_search_paths));
    args.extend(args_in.native_static_libraries.iter().cloned());
    if let Some(rpath_flag) = args_in.rpath_flag {
        args.push(rpath_flag.to_string());
    }
    args
}

fn clang_cl_jni_linker_args(args_in: &JniLinkerArgs<'_>) -> Result<Vec<String>> {
    let mut args = vec![
        "/LD".to_string(),
        args_in.jni_glue.display().to_string(),
        args_in.link_input.display().to_string(),
        format!("/I{}", args_in.jni_dir.display()),
        format!("/I{}", args_in.jni_include_directories.shared.display()),
        format!("/I{}", args_in.jni_include_directories.platform.display()),
        "/link".to_string(),
        format!("/OUT:{}", args_in.output_lib.display()),
    ];
    args.extend(msvc_rustflag_linker_args(args_in.rustflag_linker_args)?);
    args.extend(msvc_link_search_path_flags(
        args_in.native_link_search_paths,
    ));
    args.extend(msvc_native_static_library_flags(
        args_in.native_static_libraries,
    ));
    Ok(args)
}

fn msvc_link_search_path_flags(link_search_paths: &[String]) -> Vec<String> {
    let mut flags = Vec::new();

    for linked_path in link_search_paths {
        let Some(path) = linked_path
            .strip_prefix("native=")
            .or_else(|| linked_path.strip_prefix("dependency="))
            .or_else(|| linked_path.strip_prefix("all="))
            .or_else(|| linked_path.strip_prefix("crate="))
            .or_else(|| (!linked_path.starts_with("framework=")).then_some(linked_path.as_str()))
        else {
            continue;
        };

        let flag = format!("/LIBPATH:{path}");
        if !flags.contains(&flag) {
            flags.push(flag);
        }
    }

    flags
}

fn msvc_native_static_library_flags(native_static_libraries: &[String]) -> Vec<String> {
    let mut flags = Vec::new();
    let mut index = 0;

    while index < native_static_libraries.len() {
        let flag = &native_static_libraries[index];

        if flag == "-l" {
            if let Some(value) = native_static_libraries.get(index + 1) {
                flags.push(format!("{value}.lib"));
                index += 2;
                continue;
            }
        } else if let Some(value) = flag.strip_prefix("-l:") {
            flags.push(value.to_string());
            index += 1;
            continue;
        } else if let Some(value) = flag.strip_prefix("-l") {
            if !value.is_empty() {
                flags.push(format!("{value}.lib"));
                index += 1;
                continue;
            }
        } else if flag == "-framework" {
            index += 2;
            continue;
        } else {
            flags.push(flag.clone());
            index += 1;
            continue;
        }

        index += 1;
    }

    flags
}

fn msvc_rustflag_linker_args(rustflag_linker_args: &[String]) -> Result<Vec<String>> {
    let mut flags = Vec::new();
    let mut index = 0;

    while index < rustflag_linker_args.len() {
        let flag = &rustflag_linker_args[index];

        if flag == "-L" {
            if let Some(value) = rustflag_linker_args.get(index + 1) {
                flags.push(format!("/LIBPATH:{value}"));
                index += 2;
                continue;
            }
        } else if let Some(value) = flag.strip_prefix("-L") {
            if !value.is_empty() {
                flags.push(format!("/LIBPATH:{value}"));
                index += 1;
                continue;
            }
        } else if flag == "-l" {
            if let Some(value) = rustflag_linker_args.get(index + 1) {
                flags.push(format!("{value}.lib"));
                index += 2;
                continue;
            }
        } else if let Some(value) = flag.strip_prefix("-l:") {
            flags.push(value.to_string());
            index += 1;
            continue;
        } else if let Some(value) = flag.strip_prefix("-l") {
            if !value.is_empty() {
                flags.push(format!("{value}.lib"));
                index += 1;
                continue;
            }
        } else if flag == "-framework" {
            index += 2;
            continue;
        } else if flag.starts_with("-F") {
            return Err(CliError::CommandFailed {
                command: format!(
                    "unsupported Windows MSVC JNI linker arg '{}' derived from Cargo rustflags",
                    flag
                ),
                status: None,
            });
        } else if flag.starts_with('/') || flag.ends_with(".lib") || flag.ends_with(".a") {
            flags.push(flag.clone());
            index += 1;
            continue;
        } else {
            return Err(CliError::CommandFailed {
                command: format!(
                    "unsupported Windows MSVC JNI linker arg '{}' derived from Cargo rustflags",
                    flag
                ),
                status: None,
            });
        }

        index += 1;
    }

    Ok(flags)
}

fn split_toolchain_selector(cargo_args: &[String]) -> (Option<String>, Vec<String>) {
    let toolchain_selector_index = cargo_args
        .iter()
        .position(|argument| argument.starts_with('+') && argument.len() > 1);

    toolchain_selector_index
        .map(|index| {
            let toolchain_selector = cargo_args.get(index).cloned();
            let command_args = cargo_args
                .iter()
                .take(index)
                .chain(cargo_args.iter().skip(index + 1))
                .cloned()
                .collect();
            (toolchain_selector, command_args)
        })
        .unwrap_or_else(|| (None, cargo_args.to_vec()))
}

fn cargo_metadata_args(cargo_args: &[String]) -> Vec<String> {
    let mut metadata_args = Vec::new();
    let mut index = 0;

    while index < cargo_args.len() {
        let argument = &cargo_args[index];
        let takes_value = matches!(
            argument.as_str(),
            "--target-dir" | "--config" | "-Z" | "--manifest-path"
        );
        let keep_current = argument.starts_with('+')
            || takes_value
            || matches!(argument.as_str(), "--locked" | "--offline" | "--frozen")
            || argument.starts_with("--target-dir=")
            || argument.starts_with("--config=")
            || argument.starts_with("-Z")
            || argument.starts_with("--manifest-path=");

        if keep_current {
            metadata_args.push(argument.clone());
            if takes_value
                && !argument.contains('=')
                && let Some(value) = cargo_args.get(index + 1)
            {
                metadata_args.push(value.clone());
                index += 1;
            }
        }

        index += 1;
    }

    metadata_args
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::WriteFailed {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn remove_stale_flat_jvm_outputs_if_current_host_unrequested(
    java_output: &Path,
    current_host: Option<JavaHostTarget>,
    requested_host_targets: &[JavaHostTarget],
    artifact_name: &str,
) -> Result<()> {
    let Some(current_host) = current_host else {
        return Ok(());
    };

    if requested_host_targets.contains(&current_host) {
        return Ok(());
    }

    remove_file_if_exists(&java_output.join(current_host.jni_library_filename(artifact_name)))?;
    remove_file_if_exists(&java_output.join(current_host.shared_library_filename(artifact_name)))?;
    Ok(())
}

fn remove_stale_requested_jvm_shared_library_copies_after_success(
    java_output: &Path,
    packaged_outputs: &[JvmPackagedNativeOutput],
    artifact_name: &str,
) -> Result<()> {
    let current_host = JavaHostTarget::current();

    for packaged_output in packaged_outputs {
        if packaged_output.has_shared_library_copy {
            continue;
        }

        let stale_shared_library_name = packaged_output
            .host_target
            .shared_library_filename(artifact_name);
        let structured_copy = java_output
            .join("native")
            .join(packaged_output.host_target.canonical_name())
            .join(&stale_shared_library_name);
        remove_file_if_exists(&structured_copy)?;

        if current_host == Some(packaged_output.host_target) {
            remove_file_if_exists(&java_output.join(stale_shared_library_name))?;
        }
    }

    Ok(())
}

fn remove_directory_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::WriteFailed {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn remove_stale_structured_jvm_outputs(
    native_output_root: &Path,
    requested_host_targets: &[JavaHostTarget],
) -> Result<()> {
    let requested_host_directories = requested_host_targets
        .iter()
        .map(|host_target| host_target.canonical_name())
        .collect::<std::collections::HashSet<_>>();

    for host_target in [
        JavaHostTarget::DarwinArm64,
        JavaHostTarget::DarwinX86_64,
        JavaHostTarget::LinuxX86_64,
        JavaHostTarget::LinuxAarch64,
        JavaHostTarget::WindowsX86_64,
    ] {
        if requested_host_directories.contains(host_target.canonical_name()) {
            continue;
        }

        remove_directory_if_exists(&native_output_root.join(host_target.canonical_name()))?;
    }

    Ok(())
}

fn current_cargo_package_selector(cargo_args: &[String]) -> Option<String> {
    let mut package_selector = None;
    let mut index = 0;

    while index < cargo_args.len() {
        let argument = &cargo_args[index];

        if let Some(selector) = argument.strip_prefix("--package=") {
            package_selector = Some(selector.to_string());
        } else if let Some(selector) = argument.strip_prefix("-p") {
            if !selector.is_empty() {
                package_selector = Some(selector.to_string());
            } else if let Some(value) = cargo_args.get(index + 1) {
                package_selector = Some(value.clone());
                index += 1;
            }
        } else if argument == "--package"
            && let Some(value) = cargo_args.get(index + 1)
        {
            package_selector = Some(value.clone());
            index += 1;
        }

        index += 1;
    }

    package_selector
}

fn current_cargo_target_selector(cargo_args: &[String]) -> Option<String> {
    let mut target_selector = None;
    let mut index = 0;

    while index < cargo_args.len() {
        let argument = &cargo_args[index];

        if let Some(selector) = argument.strip_prefix("--target=") {
            if !selector.is_empty() {
                target_selector = Some(selector.to_string());
            }
        } else if argument == "--target"
            && let Some(value) = cargo_args.get(index + 1)
        {
            target_selector = Some(value.clone());
            index += 1;
        }

        index += 1;
    }

    target_selector
}

fn has_explicit_manifest_path(cargo_args: &[String]) -> bool {
    cargo_args
        .iter()
        .any(|argument| argument == "--manifest-path" || argument.starts_with("--manifest-path="))
}

fn effective_cargo_package_selector(
    config: &Config,
    cargo_args: &[String],
    metadata: &CargoMetadata,
    manifest_path: &Path,
) -> Option<String> {
    current_cargo_package_selector(cargo_args).or_else(|| {
        let manifest_selects_package = has_explicit_manifest_path(cargo_args)
            && metadata
                .packages
                .iter()
                .any(|package| package.manifest_path == manifest_path);

        (!manifest_selects_package)
            .then(|| {
                infer_cargo_package_selector(
                    metadata,
                    manifest_path,
                    &config.package.name,
                    &config.crate_artifact_name(),
                )
            })
            .flatten()
    })
}

fn infer_cargo_package_selector(
    metadata: &CargoMetadata,
    manifest_path: &Path,
    package_name: &str,
    crate_artifact_name: &str,
) -> Option<String> {
    metadata
        .packages
        .iter()
        .find(|package| package.manifest_path == manifest_path)
        .map(|package| package.name.clone())
        .or_else(|| {
            let mut matching_packages = metadata
                .packages
                .iter()
                .filter(|package| package.name == package_name);
            let package = matching_packages.next()?;
            matching_packages
                .next()
                .is_none()
                .then(|| package.name.clone())
        })
        .or_else(|| {
            let mut matching_packages = metadata
                .packages
                .iter()
                .filter(|package| cargo_metadata_package_has_target(package, crate_artifact_name));
            let package = matching_packages.next()?;
            matching_packages
                .next()
                .is_none()
                .then(|| package.name.clone())
        })
}

fn cargo_metadata_package_has_target(
    package: &CargoMetadataPackage,
    crate_artifact_name: &str,
) -> bool {
    package
        .targets
        .iter()
        .any(|target| target.name == crate_artifact_name)
}

fn apply_jvm_cargo_package_selection(command: &mut Command, cargo_context: &JvmCargoContext) {
    command
        .arg("--manifest-path")
        .arg(&cargo_context.cargo_manifest_path);
    if let Some(package_selector) = cargo_context.package_selector.as_deref() {
        command.arg("-p").arg(package_selector);
    }
}

fn strip_cargo_package_selectors(cargo_args: &[String]) -> Vec<String> {
    let mut filtered = Vec::new();
    let mut index = 0;

    while index < cargo_args.len() {
        let argument = &cargo_args[index];

        if argument == "--package" || argument == "-p" {
            index += 1;
            if cargo_args.get(index).is_some() {
                index += 1;
            }
            continue;
        }

        if argument.starts_with("--package=") || (argument.starts_with("-p") && argument.len() > 2)
        {
            index += 1;
            continue;
        }

        filtered.push(argument.clone());
        index += 1;
    }

    filtered
}

fn strip_cargo_manifest_path_selectors(cargo_args: &[String]) -> Vec<String> {
    let mut filtered = Vec::new();
    let mut index = 0;

    while index < cargo_args.len() {
        let argument = &cargo_args[index];

        if argument == "--manifest-path" {
            index += 1;
            if cargo_args.get(index).is_some() {
                index += 1;
            }
            continue;
        }

        if argument.starts_with("--manifest-path=") {
            index += 1;
            continue;
        }

        filtered.push(argument.clone());
        index += 1;
    }

    filtered
}

fn strip_cargo_target_selectors(cargo_args: &[String]) -> Vec<String> {
    let mut filtered = Vec::new();
    let mut index = 0;

    while index < cargo_args.len() {
        let argument = &cargo_args[index];

        if argument == "--target" {
            index += 1;
            if cargo_args.get(index).is_some() {
                index += 1;
            }
            continue;
        }

        if argument.starts_with("--target=") {
            index += 1;
            continue;
        }

        filtered.push(argument.clone());
        index += 1;
    }

    filtered
}

fn build_apple_targets(
    config: &Config,
    targets: &[RustTarget],
    release: bool,
    build_cargo_args: &[String],
    step: &Step,
) -> Result<()> {
    let on_output: Option<OutputCallback> = if step.is_verbose() {
        Some(Box::new(|line: &str| {
            print_cargo_line(line);
        }))
    } else {
        None
    };

    let build_options = BuildOptions {
        release,
        package: Some(config.library_name().to_string()),
        cargo_args: build_cargo_args.to_vec(),
        on_output,
    };
    let builder = Builder::new(config, build_options);
    let results = builder.build_targets(targets)?;

    if all_successful(&results) {
        return Ok(());
    }

    let failed = failed_targets(&results);
    Err(CliError::BuildFailed { targets: failed })
}

fn print_cargo_line(line: &str) {
    use console::style;
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("Fresh") {
        return;
    }

    if trimmed.starts_with("Compiling") {
        println!("      {}", style(trimmed).green());
    } else if trimmed.starts_with("Finished") {
        println!("      {}", style(trimmed).green().bold());
    } else if trimmed.starts_with("warning:") {
        println!("      {}", style(trimmed).yellow());
    } else if trimmed.starts_with("error") {
        println!("      {}", style(trimmed).red().bold());
    } else if trimmed.starts_with("Checking") {
        println!("      {}", style(trimmed).green());
    } else if trimmed.starts_with("Building") {
        println!("      {}", style(trimmed).cyan());
    } else {
        println!("      {}", style(trimmed).dim());
    }
}

fn build_android_targets(
    config: &Config,
    targets: &[RustTarget],
    release: bool,
    build_cargo_args: &[String],
    step: &Step,
) -> Result<()> {
    let on_output: Option<OutputCallback> = if step.is_verbose() {
        Some(Box::new(|line: &str| print_cargo_line(line)))
    } else {
        None
    };

    let build_options = BuildOptions {
        release,
        package: Some(config.library_name().to_string()),
        cargo_args: build_cargo_args.to_vec(),
        on_output,
    };
    let builder = Builder::new(config, build_options);
    let results = builder.build_android(targets)?;

    if all_successful(&results) {
        return Ok(());
    }

    let failed = failed_targets(&results);
    Err(CliError::BuildFailed { targets: failed })
}

fn build_wasm_target(
    config: &Config,
    profile: WasmProfile,
    build_cargo_args: &[String],
    step: &Step,
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
    Err(CliError::BuildFailed { targets: failed })
}

fn resolve_build_cargo_args(config: &Config, cli_cargo_args: &[String]) -> Vec<String> {
    config
        .cargo_args_for_command("build")
        .into_iter()
        .chain(cli_cargo_args.iter().cloned())
        .collect()
}

fn generate_apple_bindings(config: &Config, layout: SpmLayout, package_root: &Path) -> Result<()> {
    let swift_output_dir = match layout {
        SpmLayout::Bundled => config
            .apple_spm_wrapper_sources()
            .map(|path| package_root.join(path).join("BoltFFI"))
            .unwrap_or_else(|| package_root.join("Sources").join("BoltFFI")),
        SpmLayout::FfiOnly => package_root.join("Sources").join("BoltFFI"),
        SpmLayout::Split => config.apple_swift_output().join("BoltFFI"),
    };

    run_generate_with_output(
        config,
        GenerateOptions {
            target: GenerateTarget::Swift,
            output: Some(swift_output_dir),
            experimental: false,
        },
    )?;

    run_generate_with_output(
        config,
        GenerateOptions {
            target: GenerateTarget::Header,
            output: Some(config.apple_header_output()),
            experimental: false,
        },
    )?;

    Ok(())
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
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "npx", "tsc"]);
        cmd
    } else {
        Command::new("tsc")
    };
    command
        .arg(source_file)
        .arg("--target")
        .arg("ES2020")
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

    let output = command.output().map_err(|_| CliError::CommandFailed {
        command: "tsc".to_string(),
        status: None,
    })?;

    let module_name = config.wasm_typescript_module_name();
    let javascript_path = output_dir.join(format!("{}.js", module_name));
    let declarations_path = output_dir.join(format!("{}.d.ts", module_name));
    let emitted_outputs_exist = javascript_path.exists() && declarations_path.exists();

    if output.status.success() || emitted_outputs_exist {
        return Ok(());
    }

    Err(CliError::CommandFailed {
        command: format!("tsc failed: {}", String::from_utf8_lossy(&output.stderr)),
        status: output.status.code(),
    })
}

fn generate_wasm_loader_entrypoints(
    module_name: &str,
    enabled_targets: &[WasmNpmTarget],
    output_dir: &Path,
) -> Result<()> {
    enabled_targets
        .iter()
        .try_for_each(|target| {
            let (filename, content) = match target {
                WasmNpmTarget::Bundler => (
                    "bundler.js",
                    format!(
                        "import init from \"./{module}.js\";\nexport * from \"./{module}.js\";\nexport {{ default as init }} from \"./{module}.js\";\nexport const initialized = (async () => {{\n  const response = await fetch(new URL(\"./{module}_bg.wasm\", import.meta.url));\n  await init(response);\n}})();\n",
                        module = module_name
                    ),
                ),
                WasmNpmTarget::Web => (
                    "web.js",
                    format!(
                        "import init from \"./{module}.js\";\nexport * from \"./{module}.js\";\nexport {{ default as init }} from \"./{module}.js\";\nexport const initialized = (async () => {{\n  const response = await fetch(new URL(\"./{module}_bg.wasm\", import.meta.url));\n  await init(response);\n}})();\n",
                        module = module_name
                    ),
                ),
                WasmNpmTarget::Nodejs => (
                    "node.js",
                    format!(
                        "export * from \"./{module}_node.js\";\nexport {{ default, initialized }} from \"./{module}_node.js\";\n",
                        module = module_name
                    ),
                ),
            };

            let path = output_dir.join(filename);
            std::fs::write(&path, content).map_err(|source| CliError::WriteFailed {
                path,
                source,
            })
        })
}

fn generate_wasm_package_json(
    config: &Config,
    module_name: &str,
    enabled_targets: &[WasmNpmTarget],
    output_dir: &Path,
) -> Result<PathBuf> {
    let package_name = config
        .wasm_npm_package_name()
        .ok_or_else(|| CliError::CommandFailed {
            command: "targets.wasm.npm.package_name is required for pack wasm".to_string(),
            status: None,
        })?;
    let package_version = config
        .wasm_npm_version()
        .unwrap_or_else(|| "0.1.0".to_string());

    let has_bundler = enabled_targets.contains(&WasmNpmTarget::Bundler);
    let has_web = enabled_targets.contains(&WasmNpmTarget::Web);
    let has_node = enabled_targets.contains(&WasmNpmTarget::Nodejs);
    let default_entry = if has_bundler {
        "./bundler.js"
    } else if has_web {
        "./web.js"
    } else {
        "./node.js"
    };

    let runtime_package = config.wasm_runtime_package();
    let runtime_version = config.wasm_runtime_version();
    let mut dependencies = BTreeMap::new();
    dependencies.insert(runtime_package, runtime_version);

    let package_json = WasmPackageJson {
        name: package_name.to_string(),
        version: package_version,
        package_type: "module".to_string(),
        exports: WasmPackageExports {
            root: WasmPackageEntry {
                types: format!("./{}.d.ts", module_name),
                browser: has_web.then(|| "./web.js".to_string()),
                node: has_node.then(|| "./node.js".to_string()),
                default: default_entry.to_string(),
            },
        },
        types: format!("./{}.d.ts", module_name),
        files: vec![
            format!("{}.js", module_name),
            format!("{}.d.ts", module_name),
            format!("{}_bg.wasm", module_name),
            "bundler.js".to_string(),
            "web.js".to_string(),
            "node.js".to_string(),
        ],
        dependencies,
        license: config.wasm_npm_license(),
        repository: config.wasm_npm_repository(),
    };

    let rendered =
        serde_json::to_string_pretty(&package_json).map_err(|source| CliError::CommandFailed {
            command: format!("failed to serialize package.json: {}", source),
            status: None,
        })?;
    let package_json_path = output_dir.join("package.json");
    std::fs::write(&package_json_path, rendered).map_err(|source| CliError::WriteFailed {
        path: package_json_path.clone(),
        source,
    })?;

    Ok(package_json_path)
}

#[derive(Serialize)]
struct WasmPackageJson {
    name: String,
    version: String,
    #[serde(rename = "type")]
    package_type: String,
    exports: WasmPackageExports,
    types: String,
    files: Vec<String>,
    dependencies: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<String>,
}

#[derive(Serialize)]
struct WasmPackageExports {
    #[serde(rename = ".")]
    root: WasmPackageEntry,
}

#[derive(Serialize)]
struct WasmPackageEntry {
    types: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    browser: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<String>,
    default: String,
}

fn generate_wasm_readme(
    config: &Config,
    module_name: &str,
    enabled_targets: &[WasmNpmTarget],
    output_dir: &Path,
) -> Result<PathBuf> {
    let package_name = config.wasm_npm_package_name().unwrap_or(module_name);
    let targets_text = enabled_targets
        .iter()
        .map(|target| match target {
            WasmNpmTarget::Bundler => "bundler",
            WasmNpmTarget::Web => "web",
            WasmNpmTarget::Nodejs => "nodejs",
        })
        .collect::<Vec<_>>()
        .join(", ");
    let content = format!(
        "# {package_name}\n\nGenerated by boltffi.\n\nEnabled wasm npm targets: {targets_text}\n\n```ts\nimport {{ initialized }} from \"{package_name}\";\nawait initialized;\n```\n"
    );

    let readme_path = output_dir.join("README.md");
    std::fs::write(&readme_path, content).map_err(|source| CliError::WriteFailed {
        path: readme_path.clone(),
        source,
    })?;

    Ok(readme_path)
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct CargoMetadataPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<CargoMetadataPackageTarget>,
}

#[derive(Deserialize)]
struct CargoMetadataPackageTarget {
    name: String,
    crate_types: Vec<String>,
}

fn discover_built_libraries_for_targets(
    crate_artifact_name: &str,
    profile_directory_name: &str,
    targets: &[RustTarget],
) -> Result<Vec<BuiltLibrary>> {
    let target_directory = cargo_target_directory()?;
    Ok(BuiltLibrary::discover_for_targets(
        &target_directory,
        crate_artifact_name,
        profile_directory_name,
        targets,
    ))
}

fn missing_built_libraries(targets: &[RustTarget], libraries: &[BuiltLibrary]) -> Vec<String> {
    targets
        .iter()
        .filter(|target| libraries.iter().all(|library| library.target != **target))
        .map(|target| target.triple().to_string())
        .collect()
}

fn cargo_target_directory() -> Result<PathBuf> {
    cargo_target_directory_with_args(&[])
}

fn cargo_target_directory_with_args(cargo_args: &[String]) -> Result<PathBuf> {
    Ok(cargo_metadata_with_args(cargo_args)?.target_directory)
}

fn cargo_metadata_with_args(cargo_args: &[String]) -> Result<CargoMetadata> {
    let crate_dir = std::env::current_dir().map_err(|source| CliError::CommandFailed {
        command: format!("current_dir: {source}"),
        status: None,
    })?;
    let metadata_args = cargo_metadata_args(cargo_args);
    let (toolchain_selector, command_args) = split_toolchain_selector(&metadata_args);
    let mut command = Command::new("cargo");
    command.current_dir(&crate_dir);
    if let Some(toolchain_selector) = toolchain_selector {
        command.arg(toolchain_selector);
    }
    let output = command
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .args(&command_args)
        .output()
        .map_err(|source| CliError::CommandFailed {
            command: format!("cargo metadata: {source}"),
            status: None,
        })?;

    if !output.status.success() {
        return Err(CliError::CommandFailed {
            command: "cargo metadata --format-version 1 --no-deps".to_string(),
            status: output.status.code(),
        });
    }

    parse_cargo_metadata(&output.stdout)
}

fn current_manifest_path_with_args(cargo_args: &[String]) -> Result<PathBuf> {
    if let Some(manifest_path) = cargo_args.iter().enumerate().find_map(|(index, argument)| {
        argument
            .strip_prefix("--manifest-path=")
            .map(PathBuf::from)
            .or_else(|| {
                (argument == "--manifest-path")
                    .then(|| cargo_args.get(index + 1).map(PathBuf::from))
                    .flatten()
            })
    }) {
        let crate_dir = std::env::current_dir().map_err(|source| CliError::CommandFailed {
            command: format!("current_dir: {source}"),
            status: None,
        })?;
        let manifest_path = if manifest_path.is_absolute() {
            manifest_path
        } else {
            crate_dir.join(manifest_path)
        };

        return manifest_path
            .canonicalize()
            .map_err(|source| CliError::CommandFailed {
                command: format!(
                    "canonicalize manifest path {}: {source}",
                    manifest_path.display()
                ),
                status: None,
            });
    }

    let crate_dir = std::env::current_dir().map_err(|source| CliError::CommandFailed {
        command: format!("current_dir: {source}"),
        status: None,
    })?;

    let manifest_path = crate_dir.join("Cargo.toml");
    manifest_path
        .canonicalize()
        .map_err(|source| CliError::CommandFailed {
            command: format!(
                "canonicalize manifest path {}: {source}",
                manifest_path.display()
            ),
            status: None,
        })
}

#[cfg(test)]
fn parse_cargo_target_directory(metadata: &[u8]) -> Result<PathBuf> {
    Ok(parse_cargo_metadata(metadata)?.target_directory)
}

fn parse_cargo_metadata(metadata: &[u8]) -> Result<CargoMetadata> {
    serde_json::from_slice::<CargoMetadata>(metadata).map_err(|source| CliError::CommandFailed {
        command: format!("parse cargo metadata: {source}"),
        status: None,
    })
}

fn parse_jvm_crate_outputs(
    metadata: &CargoMetadata,
    crate_artifact_name: &str,
    manifest_path: &Path,
    package_selector: Option<&str>,
) -> Result<JvmCrateOutputs> {
    let package = find_cargo_metadata_package(metadata, manifest_path, package_selector)?;
    let target = resolve_jvm_package_library_target(package, crate_artifact_name, manifest_path)?;

    Ok(JvmCrateOutputs {
        builds_staticlib: target
            .crate_types
            .iter()
            .any(|crate_type| crate_type == "staticlib"),
        builds_cdylib: target
            .crate_types
            .iter()
            .any(|crate_type| crate_type == "cdylib"),
    })
}

fn resolve_jvm_package_artifact_name<'a>(
    package: &'a CargoMetadataPackage,
    preferred_artifact_name: &str,
    manifest_path: &Path,
) -> Result<&'a str> {
    resolve_jvm_package_library_target(package, preferred_artifact_name, manifest_path)
        .map(|target| target.name.as_str())
}

fn resolve_jvm_package_library_target<'a>(
    package: &'a CargoMetadataPackage,
    preferred_artifact_name: &str,
    manifest_path: &Path,
) -> Result<&'a CargoMetadataPackageTarget> {
    let ffi_targets = package
        .targets
        .iter()
        .filter(|target| {
            target
                .crate_types
                .iter()
                .any(|crate_type| crate_type == "staticlib" || crate_type == "cdylib")
        })
        .collect::<Vec<_>>();

    if let Some(target) = ffi_targets
        .iter()
        .copied()
        .find(|target| target.name == preferred_artifact_name)
    {
        return Ok(target);
    }

    if ffi_targets.len() == 1 {
        return Ok(ffi_targets[0]);
    }

    Err(CliError::CommandFailed {
        command: format!(
            "could not find library target '{}' in cargo metadata for '{}'",
            preferred_artifact_name,
            manifest_path.display()
        ),
        status: None,
    })
}

fn find_cargo_metadata_package<'a>(
    metadata: &'a CargoMetadata,
    manifest_path: &Path,
    package_selector: Option<&str>,
) -> Result<&'a CargoMetadataPackage> {
    if let Some(package_selector) = package_selector {
        return metadata
            .packages
            .iter()
            .find(|package| cargo_metadata_package_matches_selector(package, package_selector))
            .ok_or_else(|| CliError::CommandFailed {
                command: format!(
                    "could not find selected cargo package '{}' in cargo metadata",
                    package_selector
                ),
                status: None,
            });
    }

    metadata
        .packages
        .iter()
        .find(|package| package.manifest_path == manifest_path)
        .ok_or_else(|| CliError::CommandFailed {
            command: format!(
                "could not find current package manifest '{}' in cargo metadata",
                manifest_path.display()
            ),
            status: None,
        })
}

fn cargo_metadata_package_matches_selector(
    package: &CargoMetadataPackage,
    package_selector: &str,
) -> bool {
    package.name == package_selector
        || package.id == package_selector
        || cargo_package_spec_matches_metadata(package, package_selector)
}

fn cargo_package_spec_matches_metadata(
    package: &CargoMetadataPackage,
    package_selector: &str,
) -> bool {
    let Some((name, version)) = package_selector.rsplit_once('@') else {
        return false;
    };

    package.name == name && cargo_metadata_package_version(package).is_some_and(|v| v == version)
}

fn cargo_metadata_package_version(package: &CargoMetadataPackage) -> Option<&str> {
    let fragment = package.id.rsplit('#').next()?;
    let (_, version) = fragment.rsplit_once('@')?;
    Some(version)
}

fn existing_xcframework_checksum(config: &Config) -> Result<String> {
    let xcframework_zip = config
        .apple_xcframework_output()
        .join(format!("{}.xcframework.zip", config.xcframework_name()));

    if xcframework_zip.exists() {
        return compute_checksum(&xcframework_zip);
    }

    Err(CliError::FileNotFound(xcframework_zip))
}

fn detect_version() -> Option<String> {
    std::fs::read_to_string("Cargo.toml")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("version = "))
                .and_then(|line| {
                    line.split('=')
                        .nth(1)
                        .map(|s| s.trim().trim_matches('"').to_string())
                })
        })
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, CargoMetadataPackage, CargoMetadataPackageTarget, JniIncludeDirectories,
        JniLinkerArgs, JvmCargoContext, JvmCrateOutputs, JvmPackagedNativeOutput,
        JvmPackagingTarget, bundled_jvm_shared_library_path, cargo_metadata_args,
        clang_cl_jni_linker_args, current_cargo_package_selector, current_cargo_target_selector,
        current_manifest_path_with_args, effective_cargo_package_selector,
        ensure_java_no_build_supported, ensure_java_pack_cargo_args_supported,
        ensure_java_pack_experimental_supported, existing_jvm_shared_library_path,
        extract_library_filenames, extract_link_search_paths, extract_native_static_libraries,
        find_cargo_metadata_package, link_search_path_flags, missing_built_libraries,
        msvc_link_search_path_flags, msvc_native_static_library_flags, msvc_rustflag_linker_args,
        parse_cargo_target_directory, parse_jvm_crate_outputs, parse_native_static_libraries,
        remove_file_if_exists, remove_stale_flat_jvm_outputs_if_current_host_unrequested,
        remove_stale_requested_jvm_shared_library_copies_after_success,
        remove_stale_structured_jvm_outputs, resolve_jni_include_directories_with_overrides,
        resolve_jvm_native_link_input, select_windows_static_library_filename,
        selected_jvm_package_source_directory, split_toolchain_selector,
        strip_cargo_manifest_path_selectors, strip_cargo_package_selectors,
        strip_cargo_target_selectors, target_specific_java_home_env_key,
        target_specific_java_include_env_key,
    };
    use crate::build::CargoBuildProfile;
    use crate::config::{CargoConfig, Config, PackageConfig, TargetsConfig};
    use crate::desktop::DesktopToolchain;
    use crate::error::CliError;
    use crate::target::{BuiltLibrary, JavaHostTarget, RustTarget};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_target_directory_from_cargo_metadata() {
        let metadata = br#"{
            "packages": [],
            "workspace_members": [],
            "workspace_default_members": [],
            "resolve": null,
            "target_directory": "/tmp/boltffi-target",
            "version": 1,
            "workspace_root": "/tmp/demo"
        }"#;

        let target_directory =
            parse_cargo_target_directory(metadata).expect("expected target directory");

        assert_eq!(target_directory, PathBuf::from("/tmp/boltffi-target"));
    }

    #[test]
    fn reports_missing_built_libraries_for_unbuilt_configured_targets() {
        let libraries = vec![BuiltLibrary {
            target: RustTarget::ANDROID_ARM64,
            path: PathBuf::from("/tmp/libdemo.a"),
        }];

        let missing = missing_built_libraries(
            &[RustTarget::ANDROID_ARM64, RustTarget::ANDROID_X86_64],
            &libraries,
        );

        assert_eq!(missing, vec!["x86_64-linux-android".to_string()]);
    }

    #[test]
    fn parses_native_static_library_flags_from_cargo_output() {
        let parsed = parse_native_static_libraries(
            "note: native-static-libs: -framework Security -lresolv -lc++",
        )
        .expect("expected static library flags");

        assert_eq!(parsed, vec!["-framework", "Security", "-lresolv", "-lc++"]);
    }

    #[test]
    fn preserves_repeated_framework_prefixes_in_native_static_library_flags() {
        let parsed = parse_native_static_libraries(
            "note: native-static-libs: -framework Security -framework SystemConfiguration -lobjc",
        )
        .expect("expected static library flags");

        assert_eq!(
            parsed,
            vec![
                "-framework",
                "Security",
                "-framework",
                "SystemConfiguration",
                "-lobjc",
            ]
        );
    }

    #[test]
    fn extracts_last_native_static_library_line_from_combined_output() {
        let parsed = extract_native_static_libraries(
            "Compiling demo\nnote: native-static-libs: -lSystem\nFinished\nnote: native-static-libs: -framework CoreFoundation -lSystem\n",
        )
        .expect("expected static library flags");

        assert_eq!(parsed, vec!["-framework", "CoreFoundation", "-lSystem"]);
    }

    #[test]
    fn extracts_link_search_paths_from_build_script_messages() {
        let linked_paths = extract_link_search_paths(
            r#"{"reason":"compiler-artifact","package_id":"path+file:///tmp/demo#0.1.0"}
{"reason":"build-script-executed","package_id":"path+file:///tmp/dep#0.1.0","linked_paths":["native=/tmp/out","framework=/tmp/frameworks","native=/tmp/out"]}"#,
        );

        assert_eq!(
            linked_paths,
            vec![
                "native=/tmp/out".to_string(),
                "framework=/tmp/frameworks".to_string(),
            ]
        );
    }

    #[test]
    fn converts_link_search_paths_to_clang_flags() {
        let flags = link_search_path_flags(&[
            "native=/tmp/out".to_string(),
            "framework=/tmp/frameworks".to_string(),
            "dependency=/tmp/deps".to_string(),
            "/tmp/plain".to_string(),
            "native=/tmp/out".to_string(),
        ]);

        assert_eq!(
            flags,
            vec![
                "-L/tmp/out".to_string(),
                "-F/tmp/frameworks".to_string(),
                "-L/tmp/deps".to_string(),
                "-L/tmp/plain".to_string(),
            ]
        );
    }

    #[test]
    fn converts_link_search_paths_to_msvc_flags() {
        let flags = msvc_link_search_path_flags(&[
            "native=/tmp/out".to_string(),
            "dependency=/tmp/deps".to_string(),
            "framework=/tmp/frameworks".to_string(),
            "/tmp/plain".to_string(),
            "native=/tmp/out".to_string(),
        ]);

        assert_eq!(
            flags,
            vec![
                "/LIBPATH:/tmp/out".to_string(),
                "/LIBPATH:/tmp/deps".to_string(),
                "/LIBPATH:/tmp/plain".to_string(),
            ]
        );
    }

    #[test]
    fn converts_native_static_libraries_to_msvc_flags() {
        let flags = msvc_native_static_library_flags(&[
            "-l".to_string(),
            "bcrypt".to_string(),
            "-lws2_32".to_string(),
            "-l:custom.lib".to_string(),
            "userenv.lib".to_string(),
            "-framework".to_string(),
            "Security".to_string(),
        ]);

        assert_eq!(
            flags,
            vec![
                "bcrypt.lib".to_string(),
                "ws2_32.lib".to_string(),
                "custom.lib".to_string(),
                "userenv.lib".to_string(),
            ]
        );
    }

    #[test]
    fn converts_msvc_rustflag_linker_args() {
        let flags = msvc_rustflag_linker_args(&[
            "-L/tmp/native".to_string(),
            "-lws2_32".to_string(),
            "userenv.lib".to_string(),
            "/DEBUG".to_string(),
        ])
        .expect("msvc rustflag conversion");

        assert_eq!(
            flags,
            vec![
                "/LIBPATH:/tmp/native".to_string(),
                "ws2_32.lib".to_string(),
                "userenv.lib".to_string(),
                "/DEBUG".to_string(),
            ]
        );
    }

    #[test]
    fn rejects_unsupported_msvc_rustflag_linker_args() {
        let error = msvc_rustflag_linker_args(&["-Wl,--as-needed".to_string()])
            .expect_err("unsupported flag should fail");

        assert!(matches!(
            error,
            CliError::CommandFailed { command, status: None }
                if command.contains("-Wl,--as-needed")
        ));
    }

    #[test]
    fn builds_clang_cl_jni_linker_args_with_msvc_flags() {
        let include_directories = JniIncludeDirectories {
            shared: PathBuf::from("/tmp/jdk/include"),
            platform: PathBuf::from("/tmp/jdk/include/win32"),
        };

        let args = clang_cl_jni_linker_args(&JniLinkerArgs {
            output_lib: Path::new("/tmp/out/demo_jni.dll"),
            jni_glue: Path::new("/tmp/jni/jni_glue.c"),
            link_input: Path::new("/tmp/target/demo.lib"),
            jni_dir: Path::new("/tmp/jni"),
            jni_include_directories: &include_directories,
            rustflag_linker_args: &["-L/tmp/rustflag-native".to_string(), "-luser32".to_string()],
            native_link_search_paths: &["native=/tmp/native".to_string()],
            native_static_libraries: &["-lws2_32".to_string(), "userenv.lib".to_string()],
            rpath_flag: None,
        })
        .expect("msvc jni args");

        assert_eq!(
            args,
            vec![
                "/LD".to_string(),
                "/tmp/jni/jni_glue.c".to_string(),
                "/tmp/target/demo.lib".to_string(),
                "/I/tmp/jni".to_string(),
                "/I/tmp/jdk/include".to_string(),
                "/I/tmp/jdk/include/win32".to_string(),
                "/link".to_string(),
                "/OUT:/tmp/out/demo_jni.dll".to_string(),
                "/LIBPATH:/tmp/rustflag-native".to_string(),
                "user32.lib".to_string(),
                "/LIBPATH:/tmp/native".to_string(),
                "ws2_32.lib".to_string(),
                "userenv.lib".to_string(),
            ]
        );
    }

    #[test]
    fn rejects_pack_all_no_build_when_java_is_enabled() {
        let config = Config {
            experimental: vec!["java".to_string()],
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig {
                java: crate::config::JavaConfig {
                    jvm: crate::config::JavaJvmConfig {
                        enabled: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let error = ensure_java_no_build_supported(&config, true, false, "pack all")
            .expect_err("expected no-build rejection");

        assert!(matches!(
            error,
            CliError::CommandFailed { command, status: None }
                if command.contains("pack all --no-build is unsupported in Phase 4")
        ));
    }

    #[test]
    fn allows_pack_all_no_build_when_java_is_disabled() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };

        ensure_java_no_build_supported(&config, true, false, "pack all")
            .expect("expected no-build to be allowed");
    }

    #[test]
    fn rejects_explicit_cargo_target_for_pack_java() {
        let error = ensure_java_pack_cargo_args_supported(&[
            "--target".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        ])
        .expect_err("expected explicit target rejection");

        assert!(matches!(
            error,
            CliError::CommandFailed { command, status: None }
                if command.contains("remove cargo --target 'x86_64-unknown-linux-gnu'")
        ));
    }

    #[test]
    fn extracts_library_filenames_from_print_file_names_output() {
        let filenames = extract_library_filenames(
            "Compiling demo\nlibdemo.a\nlibdemo.dylib\nlibdemo.rlib\nFinished\n",
        );

        assert_eq!(
            filenames,
            vec![
                "libdemo.a".to_string(),
                "libdemo.dylib".to_string(),
                "libdemo.rlib".to_string(),
            ]
        );
    }

    #[test]
    fn selects_windows_static_library_filename_from_reported_outputs() {
        let filename = select_windows_static_library_filename(
            "demo",
            &[
                "demo.lib".to_string(),
                "demo.dll".to_string(),
                "demo.rlib".to_string(),
            ],
        )
        .expect("expected windows staticlib filename");

        assert_eq!(filename, "demo.lib");
    }

    #[test]
    fn selects_windows_gnu_static_library_filename_from_reported_outputs() {
        let filename = select_windows_static_library_filename(
            "demo",
            &[
                "libdemo.a".to_string(),
                "demo.dll".to_string(),
                "demo.rlib".to_string(),
            ],
        )
        .expect("expected windows gnu staticlib filename");

        assert_eq!(filename, "libdemo.a");
    }

    #[test]
    fn splits_toolchain_selector_from_cargo_args() {
        let (toolchain_selector, command_args) = split_toolchain_selector(&[
            "--features".to_string(),
            "demo".to_string(),
            "+nightly".to_string(),
            "--locked".to_string(),
        ]);

        assert_eq!(toolchain_selector.as_deref(), Some("+nightly"));
        assert_eq!(
            command_args,
            vec![
                "--features".to_string(),
                "demo".to_string(),
                "--locked".to_string()
            ]
        );
    }

    #[test]
    fn keeps_metadata_relevant_cargo_args() {
        let metadata_args = cargo_metadata_args(&[
            "+nightly".to_string(),
            "--target-dir".to_string(),
            "out/target".to_string(),
            "--config=build.target-dir=\"other-target\"".to_string(),
            "--locked".to_string(),
            "--features".to_string(),
            "demo".to_string(),
            "--manifest-path".to_string(),
            "examples/demo/Cargo.toml".to_string(),
            "-Zunstable-options".to_string(),
        ]);

        assert_eq!(
            metadata_args,
            vec![
                "+nightly".to_string(),
                "--target-dir".to_string(),
                "out/target".to_string(),
                "--config=build.target-dir=\"other-target\"".to_string(),
                "--locked".to_string(),
                "--manifest-path".to_string(),
                "examples/demo/Cargo.toml".to_string(),
                "-Zunstable-options".to_string(),
            ]
        );
    }

    #[test]
    fn canonicalizes_manifest_path_from_split_cargo_args() {
        let expected = std::env::current_dir()
            .expect("current dir")
            .join("Cargo.toml")
            .canonicalize()
            .expect("canonical manifest path");

        let manifest_path = current_manifest_path_with_args(&[
            "--manifest-path".to_string(),
            "Cargo.toml".to_string(),
        ])
        .expect("manifest path");

        assert_eq!(manifest_path, expected);
    }

    #[test]
    fn canonicalizes_manifest_path_from_equals_cargo_arg() {
        let expected = std::env::current_dir()
            .expect("current dir")
            .join("Cargo.toml")
            .canonicalize()
            .expect("canonical manifest path");

        let manifest_path =
            current_manifest_path_with_args(&["--manifest-path=Cargo.toml".to_string()])
                .expect("manifest path");

        assert_eq!(manifest_path, expected);
    }

    #[test]
    fn canonicalizes_implicit_manifest_path() {
        let expected = std::env::current_dir()
            .expect("current dir")
            .join("Cargo.toml")
            .canonicalize()
            .expect("canonical manifest path");

        let manifest_path = current_manifest_path_with_args(&[]).expect("manifest path");

        assert_eq!(manifest_path, expected);
    }

    #[test]
    fn extracts_last_package_selector_from_cargo_args() {
        let package_selector = current_cargo_package_selector(&[
            "--manifest-path".to_string(),
            "Cargo.toml".to_string(),
            "-p".to_string(),
            "first".to_string(),
            "--package=second".to_string(),
        ]);

        assert_eq!(package_selector.as_deref(), Some("second"));
    }

    #[test]
    fn extracts_package_spec_selector_from_split_cargo_args() {
        let package_selector = current_cargo_package_selector(&[
            "--locked".to_string(),
            "-p".to_string(),
            "workspace-member@1.2.3".to_string(),
        ]);

        assert_eq!(package_selector.as_deref(), Some("workspace-member@1.2.3"));
    }

    #[test]
    fn extracts_last_target_selector_from_cargo_args() {
        let target_selector = current_cargo_target_selector(&[
            "--target".to_string(),
            "aarch64-apple-darwin".to_string(),
            "--target=x86_64-unknown-linux-gnu".to_string(),
        ]);

        assert_eq!(target_selector.as_deref(), Some("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn strips_package_selectors_from_probe_cargo_args() {
        let cargo_args = strip_cargo_package_selectors(&[
            "+nightly".to_string(),
            "--package".to_string(),
            "member-a".to_string(),
            "-pmember-b".to_string(),
            "-p".to_string(),
            "member-c@1.2.3".to_string(),
            "--features".to_string(),
            "demo".to_string(),
            "--package=member-d".to_string(),
            "--release".to_string(),
        ]);

        assert_eq!(
            cargo_args,
            vec![
                "+nightly".to_string(),
                "--features".to_string(),
                "demo".to_string(),
                "--release".to_string(),
            ]
        );
    }

    #[test]
    fn strips_manifest_path_from_probe_cargo_args() {
        let cargo_args = strip_cargo_manifest_path_selectors(&[
            "--locked".to_string(),
            "--manifest-path".to_string(),
            "workspace/Cargo.toml".to_string(),
            "--manifest-path=member/Cargo.toml".to_string(),
            "--frozen".to_string(),
        ]);

        assert_eq!(
            cargo_args,
            vec!["--locked".to_string(), "--frozen".to_string()]
        );
    }

    #[test]
    fn strips_target_selectors_from_probe_cargo_args() {
        let cargo_args = strip_cargo_target_selectors(&[
            "+nightly".to_string(),
            "--target".to_string(),
            "aarch64-apple-darwin".to_string(),
            "--features".to_string(),
            "demo".to_string(),
            "--target=x86_64-unknown-linux-gnu".to_string(),
            "--release".to_string(),
        ]);

        assert_eq!(
            cargo_args,
            vec![
                "+nightly".to_string(),
                "--features".to_string(),
                "demo".to_string(),
                "--release".to_string(),
            ]
        );
    }

    #[test]
    fn builds_target_specific_java_env_keys() {
        assert_eq!(
            target_specific_java_home_env_key("x86_64-unknown-linux-gnu"),
            "BOLTFFI_JAVA_HOME_X86_64_UNKNOWN_LINUX_GNU"
        );
        assert_eq!(
            target_specific_java_include_env_key("x86_64-unknown-linux-gnu"),
            "BOLTFFI_JAVA_INCLUDE_X86_64_UNKNOWN_LINUX_GNU"
        );
    }

    #[test]
    fn falls_back_to_current_manifest_package_for_effective_package_selector() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };

        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                targets: vec![CargoMetadataPackageTarget {
                    name: "workspace_member".to_string(),
                    crate_types: vec!["cdylib".to_string()],
                }],
            }],
        };
        let package_selector = effective_cargo_package_selector(
            &config,
            &[],
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
        );

        assert_eq!(package_selector.as_deref(), Some("workspace-member"));
    }

    #[test]
    fn falls_back_to_cargo_package_name_when_crate_name_differs() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: Some("ffi_member".to_string()),
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };

        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                targets: vec![CargoMetadataPackageTarget {
                    name: "ffi_member".to_string(),
                    crate_types: vec!["cdylib".to_string()],
                }],
            }],
        };
        let package_selector = effective_cargo_package_selector(
            &config,
            &[],
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
        );

        assert_eq!(package_selector.as_deref(), Some("workspace-member"));
    }

    #[test]
    fn returns_none_for_effective_package_selector_when_manifest_path_selects_package() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/member/Cargo.toml"),
                targets: vec![],
            }],
        };

        let package_selector = effective_cargo_package_selector(
            &config,
            &[
                "--manifest-path".to_string(),
                "member/Cargo.toml".to_string(),
            ],
            &metadata,
            Path::new("/tmp/workspace/member/Cargo.toml"),
        );

        assert_eq!(package_selector, None);
    }

    #[test]
    fn falls_back_to_package_name_for_virtual_workspace_manifest_path() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/member/Cargo.toml"),
                targets: vec![CargoMetadataPackageTarget {
                    name: "workspace_member".to_string(),
                    crate_types: vec!["cdylib".to_string()],
                }],
            }],
        };

        let package_selector = effective_cargo_package_selector(
            &config,
            &[
                "--manifest-path".to_string(),
                "/tmp/workspace/Cargo.toml".to_string(),
            ],
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
        );

        assert_eq!(package_selector.as_deref(), Some("workspace-member"));
    }

    #[test]
    fn falls_back_to_package_name_when_crate_name_differs_for_virtual_workspace_manifest_path() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: Some("ffi_member".to_string()),
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/member/Cargo.toml"),
                targets: vec![CargoMetadataPackageTarget {
                    name: "ffi_member".to_string(),
                    crate_types: vec!["cdylib".to_string()],
                }],
            }],
        };

        let package_selector = effective_cargo_package_selector(
            &config,
            &[
                "--manifest-path".to_string(),
                "/tmp/workspace/Cargo.toml".to_string(),
            ],
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
        );

        assert_eq!(package_selector.as_deref(), Some("workspace-member"));
    }

    #[test]
    fn prefers_explicit_package_selector_over_config_package_name() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![],
        };

        let package_selector = effective_cargo_package_selector(
            &config,
            &["--package=selected-member".to_string()],
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
        );

        assert_eq!(package_selector.as_deref(), Some("selected-member"));
    }

    #[test]
    fn prefers_configured_package_name_over_unique_library_target_match() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: Some("ffi_member".to_string()),
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig::default(),
        };
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/other#0.1.0".to_string(),
                    name: "other-member".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/other/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "ffi_member".to_string(),
                        crate_types: vec!["cdylib".to_string()],
                    }],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                    name: "workspace-member".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/member/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "workspace_member_lib".to_string(),
                        crate_types: vec!["cdylib".to_string()],
                    }],
                },
            ],
        };

        let package_selector = effective_cargo_package_selector(
            &config,
            &[
                "--manifest-path".to_string(),
                "/tmp/workspace/Cargo.toml".to_string(),
            ],
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
        );

        assert_eq!(package_selector.as_deref(), Some("workspace-member"));
    }

    #[test]
    fn rejects_pack_java_without_experimental_gate() {
        let config = Config {
            experimental: Vec::new(),
            cargo: CargoConfig::default(),
            package: PackageConfig {
                name: "workspace-member".to_string(),
                crate_name: None,
                version: None,
                description: None,
                license: None,
                repository: None,
            },
            targets: TargetsConfig {
                java: crate::config::JavaConfig {
                    jvm: crate::config::JavaJvmConfig {
                        enabled: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let error = ensure_java_pack_experimental_supported(&config, false)
            .expect_err("expected experimental gate rejection");

        assert!(matches!(
            error,
            CliError::CommandFailed { command, status: None }
                if command.contains("java is experimental")
        ));
    }

    #[test]
    fn resolves_selected_jvm_package_source_directory_from_selected_package_manifest() {
        let current_host = JavaHostTarget::current().expect("current host");
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/member/Cargo.toml"),
                targets: vec![CargoMetadataPackageTarget {
                    name: "workspace_member".to_string(),
                    crate_types: vec!["staticlib".to_string(), "cdylib".to_string()],
                }],
            }],
        };
        let package = find_cargo_metadata_package(
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-member"),
        )
        .expect("selected package");
        let packaging_targets = vec![JvmPackagingTarget {
            cargo_context: JvmCargoContext {
                host_target: current_host,
                rust_target_triple: "x86_64-unknown-linux-gnu".to_string(),
                release: false,
                build_profile: CargoBuildProfile::Debug,
                artifact_name: "workspace_member".to_string(),
                cargo_manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                manifest_path: package.manifest_path.clone(),
                package_selector: Some("workspace-member".to_string()),
                target_directory: metadata.target_directory.clone(),
                cargo_command_args: Vec::new(),
                toolchain_selector: None,
                crate_outputs: JvmCrateOutputs {
                    builds_staticlib: true,
                    builds_cdylib: true,
                },
            },
            toolchain: DesktopToolchain::discover(None, &[], current_host, current_host)
                .expect("desktop toolchain"),
        }];

        let source_directory =
            selected_jvm_package_source_directory(&packaging_targets).expect("source directory");

        assert_eq!(source_directory, PathBuf::from("/tmp/workspace/member"));
    }

    #[test]
    fn remove_file_if_exists_deletes_existing_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-remove-file-test-{unique}"));
        fs::create_dir_all(&temp_root).expect("create temp dir");
        let file_path = temp_root.join("stale.dylib");
        fs::write(&file_path, []).expect("write temp file");

        remove_file_if_exists(&file_path).expect("remove stale file");

        assert!(!file_path.exists());

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn removes_stale_requested_shared_library_copies_only_after_success() {
        let current_host = JavaHostTarget::current();
        let requested_host = current_host.unwrap_or(JavaHostTarget::DarwinArm64);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root =
            std::env::temp_dir().join(format!("boltffi-java-requested-shared-cleanup-{unique}"));
        let native_output = temp_root
            .join("native")
            .join(requested_host.canonical_name());
        fs::create_dir_all(&native_output).expect("create structured output dir");

        let structured_shared = native_output.join(requested_host.shared_library_filename("demo"));
        fs::write(&structured_shared, []).expect("write structured shared copy");

        let flat_shared = temp_root.join(requested_host.shared_library_filename("demo"));
        if current_host == Some(requested_host) {
            fs::write(&flat_shared, []).expect("write flat shared copy");
        }

        remove_stale_requested_jvm_shared_library_copies_after_success(
            &temp_root,
            &[JvmPackagedNativeOutput {
                host_target: requested_host,
                has_shared_library_copy: false,
            }],
            "demo",
        )
        .expect("cleanup stale requested shared copies");

        assert!(!structured_shared.exists());
        if current_host == Some(requested_host) {
            assert!(!flat_shared.exists());
        }

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn removes_stale_flat_jvm_outputs_when_current_host_is_not_requested() {
        let current_host = JavaHostTarget::current().expect("supported test host");
        let requested_other_host = [
            JavaHostTarget::DarwinArm64,
            JavaHostTarget::DarwinX86_64,
            JavaHostTarget::LinuxX86_64,
            JavaHostTarget::LinuxAarch64,
            JavaHostTarget::WindowsX86_64,
        ]
        .into_iter()
        .find(|target| *target != current_host)
        .expect("alternate host");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-java-flat-cleanup-{unique}"));
        fs::create_dir_all(&temp_root).expect("create temp dir");

        let jni_copy = temp_root.join(current_host.jni_library_filename("demo"));
        let shared_copy = temp_root.join(current_host.shared_library_filename("demo"));
        fs::write(&jni_copy, []).expect("write stale jni");
        fs::write(&shared_copy, []).expect("write stale shared");

        remove_stale_flat_jvm_outputs_if_current_host_unrequested(
            &temp_root,
            Some(current_host),
            &[requested_other_host],
            "demo",
        )
        .expect("cleanup stale outputs");

        assert!(!jni_copy.exists());
        assert!(!shared_copy.exists());

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn removes_stale_structured_jvm_outputs_when_host_matrix_is_narrowed() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root =
            std::env::temp_dir().join(format!("boltffi-java-structured-cleanup-{unique}"));
        let darwin_dir = temp_root.join(JavaHostTarget::DarwinArm64.canonical_name());
        let linux_dir = temp_root.join(JavaHostTarget::LinuxX86_64.canonical_name());
        fs::create_dir_all(&darwin_dir).expect("create darwin dir");
        fs::create_dir_all(&linux_dir).expect("create linux dir");

        remove_stale_structured_jvm_outputs(&temp_root, &[JavaHostTarget::DarwinArm64])
            .expect("cleanup stale structured outputs");

        assert!(darwin_dir.exists());
        assert!(!linux_dir.exists());

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn preserves_requested_structured_jvm_outputs() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root =
            std::env::temp_dir().join(format!("boltffi-java-structured-preserve-{unique}"));
        let darwin_dir = temp_root.join(JavaHostTarget::DarwinArm64.canonical_name());
        let linux_dir = temp_root.join(JavaHostTarget::LinuxX86_64.canonical_name());
        fs::create_dir_all(&darwin_dir).expect("create darwin dir");
        fs::create_dir_all(&linux_dir).expect("create linux dir");

        remove_stale_structured_jvm_outputs(
            &temp_root,
            &[JavaHostTarget::DarwinArm64, JavaHostTarget::LinuxX86_64],
        )
        .expect("preserve structured outputs");

        assert!(darwin_dir.exists());
        assert!(linux_dir.exists());

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn preserves_flat_jvm_outputs_when_current_host_is_requested() {
        let current_host = JavaHostTarget::current().expect("supported test host");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-java-flat-preserve-{unique}"));
        fs::create_dir_all(&temp_root).expect("create temp dir");

        let jni_copy = temp_root.join(current_host.jni_library_filename("demo"));
        let shared_copy = temp_root.join(current_host.shared_library_filename("demo"));
        fs::write(&jni_copy, []).expect("write current jni");
        fs::write(&shared_copy, []).expect("write current shared");

        remove_stale_flat_jvm_outputs_if_current_host_unrequested(
            &temp_root,
            Some(current_host),
            &[current_host],
            "demo",
        )
        .expect("preserve current-host outputs");

        assert!(jni_copy.exists());
        assert!(shared_copy.exists());

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn rejects_missing_cross_host_jni_headers_during_validation() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-java-headers-test-{unique}"));
        let java_home = temp_root.join("linux-jdk");
        let shared_include = java_home.join("include");
        fs::create_dir_all(&shared_include).expect("create shared include dir");
        fs::write(shared_include.join("jni.h"), []).expect("write jni.h");

        let cargo_context = JvmCargoContext {
            host_target: JavaHostTarget::LinuxX86_64,
            rust_target_triple: "x86_64-unknown-linux-gnu".to_string(),
            release: false,
            build_profile: CargoBuildProfile::Debug,
            artifact_name: "demo".to_string(),
            cargo_manifest_path: temp_root.join("Cargo.toml"),
            manifest_path: temp_root.join("Cargo.toml"),
            package_selector: None,
            target_directory: temp_root.join("target"),
            cargo_command_args: Vec::new(),
            toolchain_selector: None,
            crate_outputs: JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            },
        };

        let error = resolve_jni_include_directories_with_overrides(
            &cargo_context,
            Some(java_home),
            None,
            None,
        )
        .expect_err("expected missing target headers error");

        assert!(matches!(
            error,
            CliError::CommandFailed { command, status: None }
                if command.contains("BOLTFFI_JAVA_INCLUDE_X86_64_UNKNOWN_LINUX_GNU")
                    && command.contains("BOLTFFI_JAVA_HOME_X86_64_UNKNOWN_LINUX_GNU")
        ));

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn rejects_missing_jni_header_files_during_validation() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root =
            std::env::temp_dir().join(format!("boltffi-java-header-files-test-{unique}"));
        let shared_include = temp_root.join("include");
        let platform_include = shared_include.join("linux");
        fs::create_dir_all(&platform_include).expect("create include dirs");

        let cargo_context = JvmCargoContext {
            host_target: JavaHostTarget::LinuxX86_64,
            rust_target_triple: "x86_64-unknown-linux-gnu".to_string(),
            release: false,
            build_profile: CargoBuildProfile::Debug,
            artifact_name: "demo".to_string(),
            cargo_manifest_path: temp_root.join("Cargo.toml"),
            manifest_path: temp_root.join("Cargo.toml"),
            package_selector: None,
            target_directory: temp_root.join("target"),
            cargo_command_args: Vec::new(),
            toolchain_selector: None,
            crate_outputs: JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            },
        };

        let error = resolve_jni_include_directories_with_overrides(
            &cargo_context,
            None,
            None,
            Some(platform_include),
        )
        .expect_err("expected missing header file error");

        assert!(matches!(error, CliError::FileNotFound(path) if path.ends_with("jni.h")));

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn accepts_target_include_override_without_java_home() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root =
            std::env::temp_dir().join(format!("boltffi-java-include-only-test-{unique}"));
        let shared_include = temp_root.join("include");
        let platform_include = shared_include.join("linux");
        fs::create_dir_all(&platform_include).expect("create platform include dir");
        fs::write(shared_include.join("jni.h"), []).expect("write jni.h");
        fs::write(platform_include.join("jni_md.h"), []).expect("write jni_md.h");

        let cargo_context = JvmCargoContext {
            host_target: JavaHostTarget::LinuxX86_64,
            rust_target_triple: "x86_64-unknown-linux-gnu".to_string(),
            release: false,
            build_profile: CargoBuildProfile::Debug,
            artifact_name: "demo".to_string(),
            cargo_manifest_path: temp_root.join("Cargo.toml"),
            manifest_path: temp_root.join("Cargo.toml"),
            package_selector: None,
            target_directory: temp_root.join("target"),
            cargo_command_args: Vec::new(),
            toolchain_selector: None,
            crate_outputs: JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            },
        };

        let include_directories = resolve_jni_include_directories_with_overrides(
            &cargo_context,
            None,
            None,
            Some(platform_include.clone()),
        )
        .expect("include override should be sufficient");

        assert_eq!(include_directories.shared, shared_include);
        assert_eq!(include_directories.platform, platform_include);

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn prefers_target_include_override_over_java_home_include() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root =
            std::env::temp_dir().join(format!("boltffi-java-include-priority-test-{unique}"));
        let host_java_home = temp_root.join("host-jdk");
        let target_include_root = temp_root.join("target-jdk").join("include");
        let target_platform_include = target_include_root.join("linux");
        fs::create_dir_all(host_java_home.join("include").join("darwin"))
            .expect("create host include dir");
        fs::create_dir_all(&target_platform_include).expect("create target include dir");
        fs::write(host_java_home.join("include").join("jni.h"), []).expect("write host jni.h");
        fs::write(target_include_root.join("jni.h"), []).expect("write target jni.h");
        fs::write(target_platform_include.join("jni_md.h"), []).expect("write target jni_md.h");

        let cargo_context = JvmCargoContext {
            host_target: JavaHostTarget::LinuxX86_64,
            rust_target_triple: "x86_64-unknown-linux-gnu".to_string(),
            release: false,
            build_profile: CargoBuildProfile::Debug,
            artifact_name: "demo".to_string(),
            cargo_manifest_path: temp_root.join("Cargo.toml"),
            manifest_path: temp_root.join("Cargo.toml"),
            package_selector: None,
            target_directory: temp_root.join("target"),
            cargo_command_args: Vec::new(),
            toolchain_selector: None,
            crate_outputs: JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            },
        };

        let include_directories = resolve_jni_include_directories_with_overrides(
            &cargo_context,
            Some(host_java_home),
            None,
            Some(target_platform_include.clone()),
        )
        .expect("target include override should take precedence");

        assert_eq!(include_directories.shared, target_include_root);
        assert_eq!(include_directories.platform, target_platform_include);

        fs::remove_dir_all(&temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn prefers_staticlib_for_jvm_linking_when_available() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-jvm-link-test-{unique}"));
        let profile_dir = temp_root.join("release");
        fs::create_dir_all(&profile_dir).expect("create profile dir");

        let staticlib = profile_dir.join("libdemo.a");
        let cdylib = profile_dir.join("libdemo.dylib");
        fs::write(&staticlib, []).expect("write staticlib");
        fs::write(&cdylib, []).expect("write cdylib");

        let resolved = resolve_jvm_native_link_input(
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            },
            Some("libdemo.a"),
        )
        .expect("expected link input");

        assert_eq!(resolved.path(), staticlib.as_path());

        fs::remove_dir_all(&temp_root).expect("cleanup temp target dir");
    }

    #[test]
    fn skips_shared_library_compatibility_copy_when_jni_links_staticlib() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-jvm-copy-test-{unique}"));
        let profile_dir = temp_root.join("release");
        fs::create_dir_all(&profile_dir).expect("create profile dir");

        let staticlib = profile_dir.join("libdemo.a");
        let cdylib = profile_dir.join("libdemo.dylib");
        fs::write(&staticlib, []).expect("write staticlib");
        fs::write(&cdylib, []).expect("write cdylib");

        let resolved = resolve_jvm_native_link_input(
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            },
            Some("libdemo.a"),
        )
        .expect("expected link input");
        let compatibility_shared_library = bundled_jvm_shared_library_path(
            &resolved,
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            },
        );

        assert_eq!(resolved.path(), staticlib.as_path());
        assert!(compatibility_shared_library.is_none());

        fs::remove_dir_all(&temp_root).expect("cleanup temp target dir");
    }

    #[test]
    fn keeps_shared_library_compatibility_copy_when_jni_links_cdylib() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-jvm-copy-cdylib-test-{unique}"));
        let profile_dir = temp_root.join("release");
        fs::create_dir_all(&profile_dir).expect("create profile dir");

        let cdylib = profile_dir.join("libdemo.dylib");
        fs::write(&cdylib, []).expect("write cdylib");

        let resolved = resolve_jvm_native_link_input(
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: false,
                builds_cdylib: true,
            },
            None,
        )
        .expect("expected link input");
        let compatibility_shared_library = bundled_jvm_shared_library_path(
            &resolved,
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: false,
                builds_cdylib: true,
            },
        )
        .expect("expected shared library compatibility copy");

        assert_eq!(resolved.path(), cdylib.as_path());
        assert_eq!(compatibility_shared_library, cdylib);

        fs::remove_dir_all(&temp_root).expect("cleanup temp target dir");
    }

    #[test]
    fn ignores_stale_staticlib_when_current_crate_is_cdylib_only() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-jvm-stale-static-{unique}"));
        let profile_dir = temp_root.join("release");
        fs::create_dir_all(&profile_dir).expect("create profile dir");

        let staticlib = profile_dir.join("libdemo.a");
        let cdylib = profile_dir.join("libdemo.dylib");
        fs::write(&staticlib, []).expect("write stale staticlib");
        fs::write(&cdylib, []).expect("write current cdylib");

        let resolved = resolve_jvm_native_link_input(
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: false,
                builds_cdylib: true,
            },
            None,
        )
        .expect("expected link input");

        assert_eq!(resolved.path(), cdylib.as_path());

        fs::remove_dir_all(&temp_root).expect("cleanup temp target dir");
    }

    #[test]
    fn ignores_stale_shared_library_when_current_crate_is_staticlib_only() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("boltffi-jvm-stale-cdylib-{unique}"));
        let profile_dir = temp_root.join("release");
        fs::create_dir_all(&profile_dir).expect("create profile dir");

        let cdylib = profile_dir.join("libdemo.dylib");
        fs::write(&cdylib, []).expect("write stale shared library");

        let compatibility_shared_library = existing_jvm_shared_library_path(
            &profile_dir,
            JavaHostTarget::DarwinArm64,
            "demo",
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            },
        );

        assert!(compatibility_shared_library.is_none());

        fs::remove_dir_all(&temp_root).expect("cleanup temp target dir");
    }

    #[test]
    fn parses_current_jvm_crate_outputs_from_cargo_metadata() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/sibling#0.1.0".to_string(),
                    name: "sibling".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/sibling/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "demo".to_string(),
                        crate_types: vec!["cdylib".to_string()],
                    }],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/current#0.1.0".to_string(),
                    name: "current".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/current/Cargo.toml"),
                    targets: vec![
                        CargoMetadataPackageTarget {
                            name: "demo".to_string(),
                            crate_types: vec![
                                "staticlib".to_string(),
                                "cdylib".to_string(),
                                "rlib".to_string(),
                            ],
                        },
                        CargoMetadataPackageTarget {
                            name: "demo_cli".to_string(),
                            crate_types: vec!["bin".to_string()],
                        },
                    ],
                },
            ],
        };

        let outputs = parse_jvm_crate_outputs(
            &metadata,
            "demo",
            Path::new("/tmp/workspace/current/Cargo.toml"),
            None,
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            }
        );
    }

    #[test]
    fn scopes_jvm_crate_outputs_to_selected_package_manifest() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/a#0.1.0".to_string(),
                    name: "workspace-a".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/a/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "shared_name".to_string(),
                        crate_types: vec!["cdylib".to_string()],
                    }],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/b#0.1.0".to_string(),
                    name: "workspace-b".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/b/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "shared_name".to_string(),
                        crate_types: vec!["staticlib".to_string()],
                    }],
                },
            ],
        };

        let outputs = parse_jvm_crate_outputs(
            &metadata,
            "shared_name",
            Path::new("/tmp/workspace/b/Cargo.toml"),
            None,
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            }
        );
    }

    #[test]
    fn finds_current_cargo_metadata_package_by_manifest_path() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/a#0.1.0".to_string(),
                    name: "workspace-a".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/a/Cargo.toml"),
                    targets: vec![],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace/b#0.1.0".to_string(),
                    name: "workspace-b".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/b/Cargo.toml"),
                    targets: vec![],
                },
            ],
        };

        let package =
            find_cargo_metadata_package(&metadata, Path::new("/tmp/workspace/b/Cargo.toml"), None)
                .expect("package lookup");

        assert_eq!(package.id, "path+file:///tmp/workspace/b#0.1.0");
    }

    #[test]
    fn finds_selected_cargo_metadata_package_by_package_name() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace#workspace-a@0.1.0".to_string(),
                    name: "workspace-a".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                    targets: vec![],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace#workspace-b@0.1.0".to_string(),
                    name: "workspace-b".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                    targets: vec![],
                },
            ],
        };

        let package = find_cargo_metadata_package(
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-b"),
        )
        .expect("package lookup");

        assert_eq!(package.id, "path+file:///tmp/workspace#workspace-b@0.1.0");
    }

    #[test]
    fn finds_selected_cargo_metadata_package_by_package_spec() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace#workspace-a@0.1.0".to_string(),
                    name: "workspace-a".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                    targets: vec![],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace#workspace-b@1.2.3".to_string(),
                    name: "workspace-b".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                    targets: vec![],
                },
            ],
        };

        let package = find_cargo_metadata_package(
            &metadata,
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-b@1.2.3"),
        )
        .expect("package lookup");

        assert_eq!(package.id, "path+file:///tmp/workspace#workspace-b@1.2.3");
    }

    #[test]
    fn scopes_jvm_crate_outputs_to_selected_package_name() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace#workspace-a@0.1.0".to_string(),
                    name: "workspace-a".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "shared_name".to_string(),
                        crate_types: vec!["cdylib".to_string()],
                    }],
                },
                CargoMetadataPackage {
                    id: "path+file:///tmp/workspace#workspace-b@0.1.0".to_string(),
                    name: "workspace-b".to_string(),
                    manifest_path: PathBuf::from("/tmp/workspace/Cargo.toml"),
                    targets: vec![CargoMetadataPackageTarget {
                        name: "shared_name".to_string(),
                        crate_types: vec!["staticlib".to_string()],
                    }],
                },
            ],
        };

        let outputs = parse_jvm_crate_outputs(
            &metadata,
            "shared_name",
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-b"),
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: false,
            }
        );
    }

    #[test]
    fn falls_back_to_selected_package_ffi_target_when_preferred_artifact_name_differs() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/tmp/boltffi-target"),
            packages: vec![CargoMetadataPackage {
                id: "path+file:///tmp/workspace/member#0.1.0".to_string(),
                name: "workspace-member".to_string(),
                manifest_path: PathBuf::from("/tmp/workspace/member/Cargo.toml"),
                targets: vec![
                    CargoMetadataPackageTarget {
                        name: "workspace_member_lib".to_string(),
                        crate_types: vec!["staticlib".to_string(), "cdylib".to_string()],
                    },
                    CargoMetadataPackageTarget {
                        name: "workspace_member_cli".to_string(),
                        crate_types: vec!["bin".to_string()],
                    },
                ],
            }],
        };

        let outputs = parse_jvm_crate_outputs(
            &metadata,
            "root_config_name",
            Path::new("/tmp/workspace/Cargo.toml"),
            Some("workspace-member"),
        )
        .expect("crate outputs");

        assert_eq!(
            outputs,
            JvmCrateOutputs {
                builds_staticlib: true,
                builds_cdylib: true,
            }
        );
    }
}
