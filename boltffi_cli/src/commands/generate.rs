use std::path::PathBuf;
use std::path::{Component, Path};

use boltffi_bindgen::render::dart::DartEmitter;
use boltffi_bindgen::render::typescript::{
    TypeScriptEmitter, TypeScriptLowerError, TypeScriptLowerer,
};
use boltffi_bindgen::{
    CfgContext, CHeaderLowerer, FactoryStyle, KotlinOptions, TypeConversion as BindgenTypeConversion,
    TypeMapping as BindgenTypeMapping, TypeMappings, ir, render, scan_crate_with_config,
};

use crate::config::{
    Config, Experimental, FactoryStyle as ConfigFactoryStyle, KotlinApiStyle, Target,
    TypeConversion as ConfigTypeConversion, WasmNpmTarget,
};
use crate::error::{CliError, Result};

pub enum GenerateTarget {
    Swift,
    Kotlin,
    Java,
    Header,
    Typescript,
    Dart,
    All,
}

pub struct GenerateOptions {
    pub target: GenerateTarget,
    pub output: Option<PathBuf>,
    pub experimental: bool,
}

fn require_experimental_target(
    config: &Config,
    target: Target,
    experimental_flag: bool,
) -> Result<()> {
    if !Experimental::is_target_experimental(target) {
        return Ok(());
    }

    let enabled_in_config = config.experimental.contains(&target.name().to_string());
    if experimental_flag || enabled_in_config {
        return Ok(());
    }

    Err(CliError::CommandFailed {
        command: format!(
            "{} is experimental, use --experimental flag or add \"{}\" to [experimental]",
            target.name(),
            target.name()
        ),
        status: None,
    })
}

pub fn run_generate_with_output(config: &Config, options: GenerateOptions) -> Result<()> {
    match options.target {
        GenerateTarget::Swift => generate_swift(config, options.output),
        GenerateTarget::Kotlin => generate_kotlin(config, options.output),
        GenerateTarget::Java => {
            require_experimental_target(config, Target::Java, options.experimental)?;
            generate_java(config, options.output)
        }
        GenerateTarget::Header => generate_header(config, options.output),
        GenerateTarget::Typescript => generate_typescript(config, options.output),
        GenerateTarget::Dart => {
            require_experimental_target(config, Target::Dart, options.experimental)?;
            generate_dart(config, options.output)
        }
        GenerateTarget::All => {
            if config.should_process(Target::Swift, options.experimental) {
                generate_swift(config, options.output.clone())?;
            }
            if config.should_process(Target::Kotlin, options.experimental) {
                generate_kotlin(config, options.output.clone())?;
            }
            if config.should_process(Target::Java, options.experimental) {
                generate_java(config, options.output.clone())?;
            }
            if config.should_process(Target::Header, options.experimental) {
                generate_header(config, options.output.clone())?;
            }
            if config.should_process(Target::TypeScript, options.experimental) {
                generate_typescript(config, options.output.clone())?;
            }
            if config.should_process(Target::Dart, options.experimental) {
                generate_dart(config, options.output)?;
            }
            Ok(())
        }
    }
}

pub fn run_generate_java_with_output_from_source_dir(
    config: &Config,
    output: Option<PathBuf>,
    experimental: bool,
    source_directory: &Path,
    crate_name: &str,
) -> Result<()> {
    require_experimental_target(config, Target::Java, experimental)?;
    generate_java_from_source_directory(config, output, source_directory, crate_name)
}

fn convert_type_mappings(
    config_mappings: &std::collections::HashMap<String, crate::config::TypeMapping>,
) -> TypeMappings {
    config_mappings
        .iter()
        .map(|(name, mapping)| {
            let conversion = match mapping.conversion {
                ConfigTypeConversion::UuidString => BindgenTypeConversion::UuidString,
                ConfigTypeConversion::UrlString => BindgenTypeConversion::UrlString,
            };
            (
                name.clone(),
                BindgenTypeMapping {
                    native_type: mapping.native_type.clone(),
                    conversion,
                },
            )
        })
        .collect()
}

fn scan_crate(
    source_directory: &Path,
    library_name: &str,
    target_pointer_width: Option<u8>,
    cfg_context: CfgContext,
) -> Result<boltffi_bindgen::Module> {
    scan_crate_with_config(
        source_directory,
        library_name,
        target_pointer_width,
        cfg_context,
    )
    .map_err(|error| CliError::CommandFailed {
        command: format!("scan_crate: {}", error),
        status: None,
    })
}

fn host_pointer_width_bits() -> Option<u8> {
    match usize::BITS {
        32 => Some(32),
        64 => Some(64),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JavaGenerationMode {
    Jvm,
    Android,
}

fn java_generation_mode(
    selected_output_dir: &Path,
    configured_jvm_output: &Path,
    configured_android_output: &Path,
    jvm_enabled: bool,
    android_enabled: bool,
) -> JavaGenerationMode {
    let selected_output_dir = normalized_output_path(selected_output_dir);
    let configured_jvm_output = normalized_output_path(configured_jvm_output);
    let configured_android_output = normalized_output_path(configured_android_output);
    if selected_output_dir == configured_jvm_output {
        return JavaGenerationMode::Jvm;
    }
    if selected_output_dir == configured_android_output {
        return JavaGenerationMode::Android;
    }
    match (jvm_enabled, android_enabled) {
        (true, false) | (true, true) => JavaGenerationMode::Jvm,
        (false, true) => JavaGenerationMode::Android,
        _ => JavaGenerationMode::Jvm,
    }
}

fn normalized_output_path(path: &Path) -> PathBuf {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current_dir| current_dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    absolute_path
        .components()
        .fold(PathBuf::new(), |mut normalized_path, component| {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if normalized_path.file_name().is_some() {
                        normalized_path.pop();
                    }
                }
                _ => normalized_path.push(component.as_os_str()),
            }
            normalized_path
        })
}

fn generate_swift(config: &Config, output: Option<PathBuf>) -> Result<()> {
    if !config.is_apple_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.apple.enabled = false".to_string(),
            status: None,
        });
    }

    let output_dir = output.unwrap_or_else(|| config.apple_swift_output());
    let library_name = config.library_name();
    let capitalized = library_name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default()
        + &library_name[1..];
    let output_path = output_dir.join(format!("{}BoltFFI.swift", capitalized));

    std::fs::create_dir_all(&output_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: output_dir.clone(),
        source,
    })?;

    let crate_dir = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    let crate_name = config.library_name();

    let mut module = scan_crate(
        &crate_dir,
        crate_name,
        Some(64),
        config.apple_scan_cfg_context(),
    )?;

    let ffi_module_name = config
        .apple_swift_ffi_module_name()
        .map(|name| name.to_string())
        .unwrap_or_else(|| format!("{}FFI", config.xcframework_name()));

    let type_mappings = convert_type_mappings(config.swift_type_mappings());

    let contract = ir::build_contract(&mut module);
    let abi_contract = ir::Lowerer::new(&contract).to_abi_contract();
    let swift_module = render::swift::SwiftLowerer::new(&contract, &abi_contract)
        .with_type_mappings(type_mappings)
        .lower()
        .map_err(|e| CliError::CommandFailed {
            command: format!("generate swift: {e}"),
            status: None,
        })?;
    let swift_code = render::swift::SwiftEmitter::with_prefix(boltffi_bindgen::ffi_prefix())
        .with_ffi_module(&ffi_module_name)
        .emit(&swift_module);

    std::fs::write(&output_path, &swift_code).map_err(|source| CliError::WriteFailed {
        path: output_path.clone(),
        source,
    })?;

    Ok(())
}

fn generate_kotlin(config: &Config, output: Option<PathBuf>) -> Result<()> {
    if !config.is_android_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.android.enabled = false".to_string(),
            status: None,
        });
    }

    let package_name = config.android_kotlin_package();
    let package_path = package_name.replace('.', "/");

    let output_dir = output.unwrap_or_else(|| config.android_kotlin_output());
    let kotlin_dir = output_dir.join(&package_path);
    let jni_dir = output_dir.join("jni");

    std::fs::create_dir_all(&kotlin_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: kotlin_dir.clone(),
        source,
    })?;
    std::fs::create_dir_all(&jni_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: jni_dir.clone(),
        source,
    })?;

    let crate_dir = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    let crate_name = config.library_name();

    let mut module = scan_crate(
        &crate_dir,
        crate_name,
        None,
        config.android_scan_cfg_context(),
    )?;

    let factory_style = match config.android_kotlin_factory_style() {
        ConfigFactoryStyle::Constructors => FactoryStyle::Constructors,
        ConfigFactoryStyle::CompanionMethods => FactoryStyle::CompanionMethods,
    };
    let module_name = config.android_kotlin_module_name();
    let kotlin_options = KotlinOptions {
        factory_style,
        api_style: match config.android_kotlin_api_style() {
            KotlinApiStyle::TopLevel => boltffi_bindgen::KotlinApiStyle::TopLevel,
            KotlinApiStyle::ModuleObject => boltffi_bindgen::KotlinApiStyle::ModuleObject,
        },
        module_object_name: Some(module_name.clone()),
        library_name: config
            .android_kotlin_library_name()
            .map(|name| name.to_string()),
        desktop_loader: true,
    };

    let type_mappings = convert_type_mappings(config.kotlin_type_mappings());

    let contract = ir::build_contract(&mut module);
    let abi_contract = ir::Lowerer::new(&contract).to_abi_contract();

    let kotlin_module = render::kotlin::KotlinLowerer::new(
        &contract,
        &abi_contract,
        package_name.clone(),
        module_name.clone(),
        kotlin_options,
    )
    .with_type_mappings(type_mappings)
    .lower();
    let kotlin_code = render::kotlin::KotlinEmitter::emit(&kotlin_module);
    let kotlin_path = kotlin_dir.join(format!("{}.kt", module_name));
    std::fs::write(&kotlin_path, &kotlin_code).map_err(|source| CliError::WriteFailed {
        path: kotlin_path.clone(),
        source,
    })?;

    let jni_module =
        render::jni::JniLowerer::new(&contract, &abi_contract, package_name, module_name)
            .with_jvm_binding_style(render::jni::JvmBindingStyle::Kotlin)
            .lower();
    let jni_code = render::jni::JniEmitter::emit(&jni_module);
    let jni_path = jni_dir.join("jni_glue.c");
    std::fs::write(&jni_path, &jni_code).map_err(|source| CliError::WriteFailed {
        path: jni_path.clone(),
        source,
    })?;

    Ok(())
}

fn generate_java(config: &Config, output: Option<PathBuf>) -> Result<()> {
    let crate_dir = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    generate_java_from_source_directory(config, output, &crate_dir, config.library_name())
}

fn generate_java_from_source_directory(
    config: &Config,
    output: Option<PathBuf>,
    source_directory: &Path,
    crate_name: &str,
) -> Result<()> {
    let jvm_enabled = config.is_java_jvm_enabled();
    let android_enabled = config.is_java_android_enabled();

    if !jvm_enabled && !android_enabled {
        return Err(CliError::CommandFailed {
            command: "both targets.java.jvm.enabled and targets.java.android.enabled are false"
                .to_string(),
            status: None,
        });
    }

    let package_name = config.java_package();
    let package_path = package_name.replace('.', "/");
    let module_name = config.java_module_name();

    let configured_jvm_output = config.java_jvm_output();
    let configured_android_output = config.java_android_output();
    let output_dir = output.unwrap_or_else(|| {
        if jvm_enabled {
            config.java_jvm_output()
        } else {
            config.java_android_output()
        }
    });
    let java_dir = output_dir.join(&package_path);
    let jni_dir = output_dir.join("jni");

    std::fs::create_dir_all(&java_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: java_dir.clone(),
        source,
    })?;
    std::fs::create_dir_all(&jni_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: jni_dir.clone(),
        source,
    })?;

    let generation_mode = java_generation_mode(
        &output_dir,
        &configured_jvm_output,
        &configured_android_output,
        jvm_enabled,
        android_enabled,
    );
    let (java_pointer_width_bits, java_cfg) = match generation_mode {
        JavaGenerationMode::Jvm => (host_pointer_width_bits(), CfgContext::default()),
        JavaGenerationMode::Android => (None, config.android_scan_cfg_context()),
    };
    let mut module = scan_crate(source_directory, crate_name, java_pointer_width_bits, java_cfg)?;

    let contract = ir::build_contract(&mut module);
    let abi_contract = ir::Lowerer::new(&contract).to_abi_contract();

    let java_options = render::java::JavaOptions {
        library_name: Some(crate_name.to_string()),
        min_java_version: render::java::JavaVersion(config.java_min_version().unwrap_or(8)),
        desktop_loader: matches!(generation_mode, JavaGenerationMode::Jvm),
    };

    let java_output = render::java::JavaEmitter::emit(
        &contract,
        &abi_contract,
        package_name.clone(),
        module_name.clone(),
        java_options,
    );

    for file in &java_output.files {
        let java_path = java_dir.join(&file.file_name);
        std::fs::write(&java_path, &file.source).map_err(|source| CliError::WriteFailed {
            path: java_path.clone(),
            source,
        })?;
    }

    let jni_module =
        render::jni::JniLowerer::new(&contract, &abi_contract, package_name, module_name)
            .with_jvm_binding_style(render::jni::JvmBindingStyle::Java)
            .lower();
    let jni_code = render::jni::JniEmitter::emit(&jni_module);
    let jni_path = jni_dir.join("jni_glue.c");
    std::fs::write(&jni_path, &jni_code).map_err(|source| CliError::WriteFailed {
        path: jni_path.clone(),
        source,
    })?;

    Ok(())
}

fn generate_header(config: &Config, output: Option<PathBuf>) -> Result<()> {
    if !config.is_apple_enabled() && !config.is_android_enabled() {
        return Err(CliError::CommandFailed {
            command: "both targets.apple.enabled and targets.android.enabled are false".to_string(),
            status: None,
        });
    }

    let output_dir = output.unwrap_or_else(|| {
        if config.is_apple_enabled() {
            config.apple_header_output()
        } else {
            config.android_header_output()
        }
    });
    let output_path = output_dir.join(format!("{}.h", config.library_name()));

    std::fs::create_dir_all(&output_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: output_dir.clone(),
        source,
    })?;

    let crate_dir = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    let crate_name = config.library_name();

    let header_pointer_width_bits = if config.is_apple_enabled() && !config.is_android_enabled() {
        Some(64)
    } else {
        None
    };
    let header_cfg = if config.is_apple_enabled() {
        config.apple_scan_cfg_context()
    } else {
        config.android_scan_cfg_context()
    };
    let mut module = scan_crate(
        &crate_dir,
        crate_name,
        header_pointer_width_bits,
        header_cfg,
    )?;

    let contract = ir::build_contract(&mut module);
    let abi = ir::Lowerer::new(&contract).to_abi_contract();
    let header_code = CHeaderLowerer::new(&contract, &abi).generate();

    std::fs::write(&output_path, header_code).map_err(|source| CliError::WriteFailed {
        path: output_path.clone(),
        source,
    })?;

    Ok(())
}

fn generate_typescript(config: &Config, output: Option<PathBuf>) -> Result<()> {
    if !config.is_wasm_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.wasm.enabled = false".to_string(),
            status: None,
        });
    }

    let experimental = config.typescript_experimental();

    let output_dir = output.unwrap_or_else(|| config.wasm_typescript_output());
    let module_name = config.wasm_typescript_module_name();
    let output_path = output_dir.join(format!("{}.ts", module_name));
    let node_output_path = output_dir.join(format!("{}_node.ts", module_name));

    std::fs::create_dir_all(&output_dir).map_err(|source| CliError::CreateDirectoryFailed {
        path: output_dir.clone(),
        source,
    })?;

    let crate_dir = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    let crate_name = config.library_name();

    let mut module = scan_crate(
        &crate_dir,
        crate_name,
        Some(32),
        config.wasm_scan_cfg_context(),
    )?;

    let contract = ir::build_contract(&mut module);
    let abi_contract = ir::Lowerer::new(&contract).to_abi_contract();

    let ts_module = TypeScriptLowerer::new(
        &contract,
        &abi_contract,
        crate_name.to_string(),
        experimental,
    )
    .lower()
    .map_err(|error| match error {
        TypeScriptLowerError::ValueTypeMemberNameCollision { .. }
        | TypeScriptLowerError::TopLevelFunctionNameCollision { .. } => CliError::CommandFailed {
            command: format!("generate typescript: {error}"),
            status: None,
        },
    })?;
    let runtime_package = config.wasm_runtime_package();

    let ts_code = TypeScriptEmitter::emit(&ts_module).replacen(
        "from \"@boltffi/runtime\"",
        &format!("from \"{}\"", runtime_package),
        1,
    );

    std::fs::write(&output_path, &ts_code).map_err(|source| CliError::WriteFailed {
        path: output_path.clone(),
        source,
    })?;

    let emits_node_bundle = config
        .wasm_npm_targets()
        .iter()
        .any(|target| matches!(target, WasmNpmTarget::Nodejs));

    if emits_node_bundle {
        let node_ts_code = TypeScriptEmitter::emit_node(&ts_module, &module_name).replacen(
            "from \"@boltffi/runtime\"",
            &format!("from \"{}\"", runtime_package),
            1,
        );

        std::fs::write(&node_output_path, &node_ts_code).map_err(|source| {
            CliError::WriteFailed {
                path: node_output_path.clone(),
                source,
            }
        })?;
    } else if node_output_path.exists() {
        std::fs::remove_file(&node_output_path).map_err(|source| CliError::WriteFailed {
            path: node_output_path.clone(),
            source,
        })?;
    }

    Ok(())
}

fn generate_dart(config: &Config, output: Option<PathBuf>) -> Result<()> {
    if !config.is_dart_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.dart.enabled = false".to_string(),
            status: None,
        });
    }

    let output_dir = output.unwrap_or_else(|| config.targets.dart.output.clone());

    if let Err(source) = std::fs::create_dir_all(&output_dir) {
        return Err(CliError::CreateDirectoryFailed {
            path: output_dir,
            source,
        });
    }

    let crate_dir = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    let crate_name = config.library_name();

    let mut module = scan_crate(
        &crate_dir,
        crate_name,
        Some(32),
        config.wasm_scan_cfg_context(),
    )?;

    let ffi = ir::build_contract(&mut module);
    let abi = ir::Lowerer::new(&ffi).to_abi_contract();

    let output = DartEmitter::emit(&ffi, &abi, &config.package.name);

    let output_path = output_dir.join(format!("{}.dart", config.package.name));

    if let Err(source) = std::fs::write(&output_path, &output) {
        return Err(CliError::WriteFailed {
            path: output_path,
            source,
        });
    }

    Ok(())
}
