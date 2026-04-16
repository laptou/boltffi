use boltffi_bindgen::render::jni::{JniEmitter, JniLowerer, JvmBindingStyle};
use boltffi_bindgen::render::kotlin::{KotlinEmitter, KotlinLowerer};
use boltffi_bindgen::{
    FactoryStyle as BindgenFactoryStyle, KotlinApiStyle as BindgenKotlinApiStyle, KotlinOptions,
};

use crate::cli::{CliError, Result};
use crate::commands::generate::generator::{
    GenerateRequest, LanguageGenerator, ScanPointerWidth, bindgen_type_mappings,
};
use crate::config::{
    FactoryStyle as ConfigFactoryStyle, KotlinApiStyle as ConfigKotlinApiStyle, Target,
};

pub struct KotlinGenerator;

impl KotlinGenerator {
    fn kotlin_options(request: &GenerateRequest<'_>, module_name: &str) -> KotlinOptions {
        let factory_style = match request.config().android_kotlin_factory_style() {
            ConfigFactoryStyle::Constructors => BindgenFactoryStyle::Constructors,
            ConfigFactoryStyle::CompanionMethods => BindgenFactoryStyle::CompanionMethods,
        };

        KotlinOptions {
            factory_style,
            api_style: match request.config().android_kotlin_api_style() {
                ConfigKotlinApiStyle::TopLevel => BindgenKotlinApiStyle::TopLevel,
                ConfigKotlinApiStyle::ModuleObject => BindgenKotlinApiStyle::ModuleObject,
            },
            module_object_name: Some(module_name.to_string()),
            library_name: request
                .config()
                .android_kotlin_library_name()
                .map(boltffi_bindgen::library_name),
            desktop_loader: true,
        }
    }
}

impl LanguageGenerator for KotlinGenerator {
    const TARGET: Target = Target::Kotlin;

    fn generate(request: &GenerateRequest<'_>) -> Result<()> {
        if !request.config().is_android_enabled() {
            return Err(CliError::CommandFailed {
                command: "targets.android.enabled = false".to_string(),
                status: None,
            });
        }

        let package_name = request.config().android_kotlin_package();
        let package_path = package_name.replace('.', "/");
        let output_directory = request
            .output_override()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| request.config().android_kotlin_output());
        let kotlin_directory = output_directory.join(&package_path);
        let jni_directory = output_directory.join("jni");

        request.ensure_output_directory(&kotlin_directory)?;
        request.ensure_output_directory(&jni_directory)?;

        let lowered_crate = request.lowered_crate(ScanPointerWidth::Flexible)?;
        let module_name = request.config().android_kotlin_module_name();
        let kotlin_module = KotlinLowerer::new(
            &lowered_crate.ffi_contract,
            &lowered_crate.abi_contract,
            package_name.clone(),
            module_name.clone(),
            Self::kotlin_options(request, &module_name),
        )
        .with_type_mappings(bindgen_type_mappings(
            request.config().kotlin_type_mappings(),
        ))
        .lower();
        let kotlin_source = KotlinEmitter::emit(&kotlin_module);
        let kotlin_path = kotlin_directory.join(format!("{module_name}.kt"));

        request.write_output(&kotlin_path, kotlin_source)?;

        let jni_module = JniLowerer::new(
            &lowered_crate.ffi_contract,
            &lowered_crate.abi_contract,
            package_name,
            module_name,
        )
        .with_jvm_binding_style(JvmBindingStyle::Kotlin)
        .lower();
        let jni_source = JniEmitter::emit(&jni_module);
        let jni_path = jni_directory.join("jni_glue.c");

        request.write_output(&jni_path, jni_source)
    }
}
