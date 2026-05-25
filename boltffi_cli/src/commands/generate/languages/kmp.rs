use std::path::{Path, PathBuf};

use boltffi_bindgen::KotlinOptions;
use boltffi_bindgen::render::kmp::{KMPEmitter, KMPOptions};
use boltffi_bindgen::render::kotlin::{
    FactoryStyle as BindgenFactoryStyle, KotlinApiStyle, KotlinDesktopLoader,
};

use crate::cli::{CliError, Result};
use crate::commands::generate::generator::SourceCrate;
use crate::commands::generate::generator::{GenerateRequest, LanguageGenerator, ScanPointerWidth};
use crate::config::{FactoryStyle as ConfigFactoryStyle, Target};

pub struct KMPGenerator;

impl KMPGenerator {
    pub fn generate_from_source_directory_with_desktop_fallback_library_name(
        config: &crate::config::Config,
        output_override: Option<PathBuf>,
        source_directory: &Path,
        crate_name: &str,
        desktop_fallback_library_name: Option<&str>,
    ) -> Result<()> {
        let request = GenerateRequest::new(
            config,
            output_override,
            SourceCrate::new(source_directory, crate_name),
        );

        Self::generate_with_desktop_fallback_library_name(&request, desktop_fallback_library_name)
    }

    fn kotlin_options(
        request: &GenerateRequest<'_>,
        module_name: &str,
        desktop_fallback_library_name: Option<&str>,
    ) -> KotlinOptions {
        let factory_style = match request.config().android_kotlin_factory_style() {
            ConfigFactoryStyle::Constructors => BindgenFactoryStyle::Constructors,
            ConfigFactoryStyle::CompanionMethods => BindgenFactoryStyle::CompanionMethods,
        };
        let desktop_fallback_library_name =
            desktop_fallback_library_name.unwrap_or_else(|| request.source_crate().crate_name());

        KotlinOptions {
            factory_style,
            api_style: KotlinApiStyle::TopLevel,
            module_object_name: Some(module_name.to_string()),
            library_name: Some(boltffi_bindgen::load_library_name(
                &request.config().resolved_android_kotlin_library_name(),
            )),
            desktop_jni_library_name: Some(boltffi_bindgen::library_name(
                &request
                    .config()
                    .resolved_android_kotlin_desktop_library_name(),
            )),
            desktop_fallback_library_name: Some(boltffi_bindgen::library_name(
                desktop_fallback_library_name,
            )),
            desktop_loader: KotlinDesktopLoader::Bundled,
        }
    }

    fn generate_with_desktop_fallback_library_name(
        request: &GenerateRequest<'_>,
        desktop_fallback_library_name: Option<&str>,
    ) -> Result<()> {
        if !request.config().is_kotlin_multiplatform_enabled() {
            return Err(CliError::CommandFailed {
                command: "targets.kotlin_multiplatform.enabled = false".to_string(),
                status: None,
            });
        }

        let output_directory = request
            .output_override()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| request.config().kotlin_multiplatform_output());
        request.ensure_output_directory(&output_directory)?;

        let lowered_crate = request.lowered_crate(ScanPointerWidth::Flexible)?;
        let module_name = request.config().kotlin_multiplatform_module_name();
        let kmp_output = KMPEmitter::emit(
            &lowered_crate.ffi_contract,
            &lowered_crate.abi_contract,
            KMPOptions {
                package_name: request.config().kotlin_multiplatform_package(),
                module_name: module_name.clone(),
                min_sdk: request.config().android_min_sdk(),
                kotlin_options: Self::kotlin_options(
                    request,
                    &module_name,
                    desktop_fallback_library_name,
                ),
            },
        );

        kmp_output.files.iter().try_for_each(|output_file| {
            let output_path = output_directory.join(&output_file.relative_path);

            if let Some(parent_directory) = output_path.parent() {
                request.ensure_output_directory(parent_directory)?;
            }

            request.write_output(&output_path, &output_file.contents)
        })
    }
}

impl LanguageGenerator for KMPGenerator {
    const TARGET: Target = Target::KotlinMultiplatform;

    fn generate(request: &GenerateRequest<'_>) -> Result<()> {
        Self::generate_with_desktop_fallback_library_name(request, None)
    }
}
