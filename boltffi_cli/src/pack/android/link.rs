use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::{CliError, Result};
use crate::config::Config;
use crate::pack::PackError;
use crate::target::{BuiltLibrary, Platform, RustTarget};
use crate::toolchain::AndroidToolchain;

pub struct AndroidPackager<'a> {
    config: &'a Config,
    libraries: Vec<BuiltLibrary>,
    release: bool,
}

pub struct AndroidOutput;

impl<'a> AndroidPackager<'a> {
    pub fn new(config: &'a Config, libraries: Vec<BuiltLibrary>, release: bool) -> Self {
        Self {
            config,
            libraries,
            release,
        }
    }

    pub fn package(self) -> Result<AndroidOutput> {
        let android_libs = self.filter_android_libraries();

        if android_libs.is_empty() {
            return Err(PackError::NoLibrariesFound {
                platform: "Android".to_string(),
            }
            .into());
        }

        let jnilibs_path = self.config.android_pack_output();
        let android_toolchain = AndroidToolchain::discover(
            self.config.android_min_sdk(),
            self.config.android_ndk_version(),
        )?;

        std::fs::create_dir_all(&jnilibs_path).map_err(|source| {
            CliError::CreateDirectoryFailed {
                path: jnilibs_path.clone(),
                source,
            }
        })?;

        let jni_glue_path = self.android_jni_glue_path()?;
        let header_include_dir = self.config.android_header_output();
        let header_path = header_include_dir.join(format!("{}.h", self.config.library_name()));
        if !header_path.exists() {
            return Err(CliError::FileNotFound(header_path));
        }

        for lib in &android_libs {
            self.link_shared_library(
                lib,
                &jnilibs_path,
                &android_toolchain,
                &jni_glue_path,
                &header_include_dir,
            )?;
        }

        self.remove_stale_packaged_libraries(&jnilibs_path, &android_libs)?;

        Ok(AndroidOutput)
    }

    fn remove_stale_packaged_libraries(
        &self,
        jnilibs_path: &Path,
        android_libs: &[&BuiltLibrary],
    ) -> Result<()> {
        let packaged_triples: std::collections::HashSet<_> = android_libs
            .iter()
            .map(|library| library.target.triple())
            .collect();
        let lib_file_name = format!("lib{}.so", self.config.library_name());

        for target in RustTarget::ALL_ANDROID {
            if packaged_triples.contains(target.triple()) {
                continue;
            }

            let stale_output = jnilibs_path
                .join(target.architecture().android_abi())
                .join(&lib_file_name);
            if stale_output.exists() {
                std::fs::remove_file(&stale_output).map_err(|source| CliError::CommandFailed {
                    command: format!("remove stale android library {}", stale_output.display()),
                    status: source.raw_os_error(),
                })?;
            }
        }

        Ok(())
    }

    fn filter_android_libraries(&self) -> Vec<&BuiltLibrary> {
        self.libraries
            .iter()
            .filter(|lib| lib.target.platform() == Platform::Android)
            .collect()
    }

    fn android_jni_glue_path(&self) -> Result<PathBuf> {
        let jni_glue_path = self
            .config
            .android_kotlin_output()
            .join("jni")
            .join("jni_glue.c");
        jni_glue_path
            .exists()
            .then_some(jni_glue_path.clone())
            .ok_or(CliError::FileNotFound(jni_glue_path))
    }

    fn link_shared_library(
        &self,
        library: &BuiltLibrary,
        jnilibs_path: &Path,
        android_toolchain: &AndroidToolchain,
        jni_glue_path: &Path,
        header_include_dir: &Path,
    ) -> Result<PathBuf> {
        let abi = library.target.architecture().android_abi();
        let abi_dir = jnilibs_path.join(abi);

        std::fs::create_dir_all(&abi_dir).map_err(|source| CliError::CreateDirectoryFailed {
            path: abi_dir.clone(),
            source,
        })?;

        let lib_name = self.config.library_name();
        let dest_path = abi_dir.join(format!("lib{}.so", lib_name));
        let build_dir = PathBuf::from("target")
            .join("boltffi")
            .join("android")
            .join(library.target.triple())
            .join(if self.release { "release" } else { "debug" });
        std::fs::create_dir_all(&build_dir).map_err(|source| CliError::CreateDirectoryFailed {
            path: build_dir.clone(),
            source,
        })?;

        let clang = android_toolchain.clang_for_target(&library.target)?;
        let object_path = build_dir.join("jni_glue.o");

        let mut compile = Command::new(&clang);
        compile
            .arg("-c")
            .arg("-fPIC")
            .arg(if self.release { "-O3" } else { "-O0" })
            .arg("-I")
            .arg(header_include_dir)
            .arg(jni_glue_path)
            .arg("-o")
            .arg(&object_path);
        run_command(compile)?;

        let mut link = Command::new(&clang);
        link.args(android_shared_link_args(
            &dest_path,
            &object_path,
            &library.path,
        ));
        run_command(link)?;

        Ok(dest_path)
    }
}

fn android_shared_link_args(
    dest_path: &Path,
    object_path: &Path,
    library_path: &Path,
) -> Vec<OsString> {
    vec![
        OsString::from("-shared"),
        OsString::from("-o"),
        dest_path.as_os_str().to_os_string(),
        object_path.as_os_str().to_os_string(),
        OsString::from("-Wl,--whole-archive"),
        library_path.as_os_str().to_os_string(),
        OsString::from("-Wl,--no-whole-archive"),
        OsString::from("-Wl,--exclude-libs,ALL"),
        OsString::from("-Wl,--gc-sections"),
        OsString::from("-lm"),
        OsString::from("-llog"),
        OsString::from("-ldl"),
    ]
}

fn run_command(mut command: Command) -> Result<()> {
    let command_string = format!("{:?}", command);
    let status = command.status().map_err(|_| CliError::CommandFailed {
        command: command_string.clone(),
        status: None,
    })?;

    status
        .success()
        .then_some(())
        .ok_or(CliError::CommandFailed {
            command: command_string,
            status: status.code(),
        })
}

#[cfg(test)]
mod tests {
    use super::{AndroidPackager, android_shared_link_args};
    use crate::config::Config;
    use crate::target::{BuiltLibrary, RustTarget};
    use std::ffi::OsString;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse_config(input: &str) -> Config {
        let parsed: Config = toml::from_str(input).expect("toml parse failed");
        parsed.validate().expect("config validation failed");
        parsed
    }

    #[test]
    fn stale_cleanup_removes_only_boltffi_library_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("boltffi-android-packager-test-{unique}"));
        let pack_output = root.join("jniLibs");
        let config = parse_config(&format!(
            r#"
[package]
name = "demo"

[targets.android.pack]
output = "{}"
"#,
            pack_output.display()
        ));

        let stale_abi_dir = pack_output.join("x86");
        fs::create_dir_all(&stale_abi_dir).expect("create stale abi dir");
        let stale_boltffi = stale_abi_dir.join("libdemo.so");
        let unrelated = stale_abi_dir.join("libdependency.so");
        fs::write(&stale_boltffi, []).expect("write stale boltffi lib");
        fs::write(&unrelated, []).expect("write unrelated lib");

        let arm64_abi_dir = pack_output.join("arm64-v8a");
        fs::create_dir_all(&arm64_abi_dir).expect("create configured abi dir");
        let packager = AndroidPackager::new(
            &config,
            vec![BuiltLibrary {
                target: RustTarget::ANDROID_ARM64,
                path: root.join("libdemo.a"),
            }],
            false,
        );
        let android_libs = packager.filter_android_libraries();

        packager
            .remove_stale_packaged_libraries(&pack_output, &android_libs)
            .expect("cleanup succeeds");

        assert!(!stale_boltffi.exists());
        assert!(unrelated.exists());

        fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    #[test]
    fn android_linker_hides_archive_symbols_and_collects_unused_sections() {
        let args = android_shared_link_args(
            Path::new("/tmp/out/libdemo.so"),
            Path::new("/tmp/out/jni_glue.o"),
            Path::new("/tmp/out/libdemo.a"),
        );

        assert!(args.contains(&OsString::from("-Wl,--exclude-libs,ALL")));
        assert!(args.contains(&OsString::from("-Wl,--gc-sections")));
    }

    #[cfg(unix)]
    #[test]
    fn android_linker_preserves_non_utf8_paths() {
        let dest_path = PathBuf::from(OsString::from_vec(b"/tmp/out-\xFF.so".to_vec()));
        let object_path = PathBuf::from(OsString::from_vec(b"/tmp/jni-\xFE.o".to_vec()));
        let library_path = PathBuf::from(OsString::from_vec(b"/tmp/lib-\xFD.a".to_vec()));
        let args = android_shared_link_args(&dest_path, &object_path, &library_path);

        assert_eq!(
            args[2].as_os_str().as_bytes(),
            dest_path.as_os_str().as_bytes()
        );
        assert_eq!(
            args[3].as_os_str().as_bytes(),
            object_path.as_os_str().as_bytes()
        );
        assert_eq!(
            args[5].as_os_str().as_bytes(),
            library_path.as_os_str().as_bytes()
        );
    }
}
