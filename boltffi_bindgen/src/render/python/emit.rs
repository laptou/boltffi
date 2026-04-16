use std::path::PathBuf;

use askama::Template as _;

use crate::render::python::PythonModule;
use crate::render::python::templates::{
    InitStubTemplate, InitTemplate, NativeModuleTemplate, PyprojectTemplate, SetupTemplate,
};

pub struct PythonEmitter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonOutputFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonPackageSources {
    pub files: Vec<PythonOutputFile>,
}

impl PythonEmitter {
    pub fn emit(module: &PythonModule) -> PythonPackageSources {
        let package_directory = PathBuf::from(&module.module_name);
        let package_version_literal =
            format!("{:?}", module.package_version.as_deref().unwrap_or("0.0.0"));
        let native_extension_name_literal =
            format!("{:?}", format!("{}._native", module.module_name));
        let native_source_path_literal =
            format!("{:?}", format!("{}/_native.c", module.module_name));
        let used_scalar_types = module.used_scalar_types();

        PythonPackageSources {
            files: vec![
                PythonOutputFile {
                    relative_path: PathBuf::from("pyproject.toml"),
                    contents: rendered_text_file(PyprojectTemplate.render().unwrap()),
                },
                PythonOutputFile {
                    relative_path: PathBuf::from("setup.py"),
                    contents: SetupTemplate {
                        module,
                        package_version_literal: &package_version_literal,
                        native_extension_name_literal: &native_extension_name_literal,
                        native_source_path_literal: &native_source_path_literal,
                    }
                    .render()
                    .unwrap(),
                },
                PythonOutputFile {
                    relative_path: package_directory.join("__init__.py"),
                    contents: InitTemplate { module }.render().unwrap(),
                },
                PythonOutputFile {
                    relative_path: package_directory.join("__init__.pyi"),
                    contents: InitStubTemplate { module }.render().unwrap(),
                },
                PythonOutputFile {
                    relative_path: package_directory.join("py.typed"),
                    contents: String::new(),
                },
                PythonOutputFile {
                    relative_path: package_directory.join("_native.c"),
                    contents: NativeModuleTemplate {
                        module,
                        used_scalar_types: &used_scalar_types,
                    }
                    .render()
                    .unwrap(),
                },
            ],
        }
    }
}

fn rendered_text_file(contents: String) -> String {
    if contents.ends_with('\n') {
        contents
    } else {
        format!("{contents}\n")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PythonEmitter;
    use crate::ir::types::PrimitiveType;
    use crate::render::python::{PythonFunction, PythonModule, PythonParameter, PythonType};

    fn rendered_file<'a>(
        rendered: &'a super::PythonPackageSources,
        relative_path: &str,
    ) -> &'a str {
        rendered
            .files
            .iter()
            .find(|file| file.relative_path == Path::new(relative_path))
            .map(|file| file.contents.as_str())
            .expect("expected generated file")
    }

    #[test]
    fn emits_native_scalar_python_package_sources() {
        let module = PythonModule {
            module_name: "demo_lib".to_string(),
            package_name: "demo-lib".to_string(),
            package_version: Some("0.1.0".to_string()),
            library_name: "demo".to_string(),
            functions: vec![
                PythonFunction {
                    python_name: "echo_i32".to_string(),
                    ffi_symbol: "boltffi_echo_i32".to_string(),
                    parameters: vec![PythonParameter {
                        name: "value".to_string(),
                        type_ref: PythonType::Primitive(PrimitiveType::I32),
                    }],
                    return_type: PythonType::Primitive(PrimitiveType::I32),
                },
                PythonFunction {
                    python_name: "echo_bool".to_string(),
                    ffi_symbol: "boltffi_echo_bool".to_string(),
                    parameters: vec![PythonParameter {
                        name: "value".to_string(),
                        type_ref: PythonType::Primitive(PrimitiveType::Bool),
                    }],
                    return_type: PythonType::Primitive(PrimitiveType::Bool),
                },
                PythonFunction {
                    python_name: "echo_f32".to_string(),
                    ffi_symbol: "boltffi_echo_f32".to_string(),
                    parameters: vec![PythonParameter {
                        name: "value".to_string(),
                        type_ref: PythonType::Primitive(PrimitiveType::F32),
                    }],
                    return_type: PythonType::Primitive(PrimitiveType::F32),
                },
            ],
        };

        let rendered = PythonEmitter::emit(&module);
        let pyproject_source = rendered_file(&rendered, "pyproject.toml");
        let setup_source = rendered_file(&rendered, "setup.py");
        let init_source = rendered_file(&rendered, "demo_lib/__init__.py");
        let native_source = rendered_file(&rendered, "demo_lib/_native.c");

        assert!(pyproject_source.contains("build-backend = \"setuptools.build_meta\""));
        assert!(pyproject_source.ends_with('\n'));
        assert!(setup_source.contains("Extension("));
        assert!(setup_source.contains("\"demo_lib._native\""));
        assert!(setup_source.contains("\"*.pyi\""));
        assert!(init_source.contains("from pathlib import Path"));
        assert!(init_source.contains("from . import _native"));
        assert!(init_source.contains("_native._initialize_loader"));
        assert!(init_source.contains("PACKAGE_NAME = \"demo-lib\""));
        assert!(
            native_source
                .contains("typedef int32_t (*boltffi_python_echo_i32_symbol_fn)(int32_t);")
        );
        assert!(native_source.contains("static PyObject *boltffi_python_initialize_loader"));
        assert!(native_source.contains("static int boltffi_python_parse_i32"));
        assert!(native_source.contains("static PyObject *boltffi_python_echo_bool"));
        assert!(native_source.contains("wchar_t *wide_library_path = NULL;"));
        assert!(native_source.contains("dlsym"));
        assert!(native_source.contains("GetProcAddress"));
        assert!(native_source.contains("FLT_MAX"));
        assert!(!native_source.contains("isfinite"));
        assert!(native_source.contains("METH_FASTCALL"));
        assert!(pyproject_source.contains("wheel>=0.43"));
    }
}
