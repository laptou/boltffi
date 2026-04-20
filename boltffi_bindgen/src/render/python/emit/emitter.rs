use std::path::PathBuf;

use askama::Template as _;

use crate::render::python::PythonModule;
use crate::render::python::templates::{
    InitStubTemplate, InitTemplate, NativeModuleTemplate, PyprojectTemplate, SetupTemplate,
};

use super::{PythonOutputFile, PythonPackageSources};

pub struct PythonEmitter;

impl PythonEmitter {
    pub fn emit(module: &PythonModule) -> PythonPackageSources {
        let package_directory = PathBuf::from(&module.module_name);
        let package_version_literal =
            format!("{:?}", module.package_version.as_deref().unwrap_or("0.0.0"));
        let native_extension_name_literal =
            format!("{:?}", format!("{}._native", module.module_name));
        let native_source_path_literal =
            format!("{:?}", format!("{}/_native.c", module.module_name));
        let used_primitive_types = module.used_primitive_types();

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
                        used_primitive_types: &used_primitive_types,
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
    use super::PythonPackageSources;
    use crate::ir::types::PrimitiveType;
    use crate::render::python::{
        PythonCStyleEnum, PythonCStyleEnumVariant, PythonCallable, PythonEnumConstructor,
        PythonEnumMethod, PythonEnumType, PythonFunction, PythonModule, PythonParameter,
        PythonSequenceType, PythonType,
    };

    struct NativePythonPackageFixture;

    impl NativePythonPackageFixture {
        fn module() -> PythonModule {
            let status_enum = PythonCStyleEnum {
                type_ref: PythonEnumType {
                    native_name_stem: "status".to_string(),
                    class_name: "Status".to_string(),
                    tag_type: PrimitiveType::I32,
                },
                variants: vec![
                    PythonCStyleEnumVariant {
                        member_name: "ACTIVE".to_string(),
                        native_value: 0,
                        native_c_literal: "((int32_t)0)".to_string(),
                        wire_tag: 0,
                        wire_c_literal: "((int32_t)0)".to_string(),
                        doc: None,
                    },
                    PythonCStyleEnumVariant {
                        member_name: "INACTIVE".to_string(),
                        native_value: 1,
                        native_c_literal: "((int32_t)1)".to_string(),
                        wire_tag: 1,
                        wire_c_literal: "((int32_t)1)".to_string(),
                        doc: None,
                    },
                ],
                constructors: vec![],
                methods: vec![],
            };
            let direction_enum = PythonCStyleEnum {
                type_ref: PythonEnumType {
                    native_name_stem: "direction".to_string(),
                    class_name: "Direction".to_string(),
                    tag_type: PrimitiveType::I32,
                },
                variants: vec![
                    PythonCStyleEnumVariant {
                        member_name: "NORTH".to_string(),
                        native_value: 0,
                        native_c_literal: "((int32_t)0)".to_string(),
                        wire_tag: 0,
                        wire_c_literal: "((int32_t)0)".to_string(),
                        doc: None,
                    },
                    PythonCStyleEnumVariant {
                        member_name: "SOUTH".to_string(),
                        native_value: 1,
                        native_c_literal: "((int32_t)1)".to_string(),
                        wire_tag: 1,
                        wire_c_literal: "((int32_t)1)".to_string(),
                        doc: None,
                    },
                ],
                constructors: vec![PythonEnumConstructor {
                    python_name: "new".to_string(),
                    callable: PythonCallable {
                        native_name: "_boltffi_direction_new".to_string(),
                        ffi_symbol: "boltffi_direction_new".to_string(),
                        parameters: vec![PythonParameter {
                            name: "raw".to_string(),
                            type_ref: PythonType::Primitive(PrimitiveType::I32),
                        }],
                        return_type: PythonType::CStyleEnum(PythonEnumType {
                            native_name_stem: "direction".to_string(),
                            class_name: "Direction".to_string(),
                            tag_type: PrimitiveType::I32,
                        }),
                    },
                }],
                methods: vec![PythonEnumMethod {
                    python_name: "opposite".to_string(),
                    callable: PythonCallable {
                        native_name: "_boltffi_direction_opposite".to_string(),
                        ffi_symbol: "boltffi_direction_opposite".to_string(),
                        parameters: vec![PythonParameter {
                            name: "self".to_string(),
                            type_ref: PythonType::CStyleEnum(PythonEnumType {
                                native_name_stem: "direction".to_string(),
                                class_name: "Direction".to_string(),
                                tag_type: PrimitiveType::I32,
                            }),
                        }],
                        return_type: PythonType::CStyleEnum(PythonEnumType {
                            native_name_stem: "direction".to_string(),
                            class_name: "Direction".to_string(),
                            tag_type: PrimitiveType::I32,
                        }),
                    },
                    is_static: false,
                }],
            };

            PythonModule {
                module_name: "demo_lib".to_string(),
                package_name: "demo-lib".to_string(),
                package_version: Some("0.1.0".to_string()),
                library_name: "demo".to_string(),
                free_buffer_symbol: "boltffi_free_buf".to_string(),
                enums: vec![status_enum, direction_enum],
                functions: vec![
                    PythonFunction {
                        python_name: "echo_i32".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_i32".to_string(),
                            ffi_symbol: "boltffi_echo_i32".to_string(),
                            parameters: vec![PythonParameter {
                                name: "value".to_string(),
                                type_ref: PythonType::Primitive(PrimitiveType::I32),
                            }],
                            return_type: PythonType::Primitive(PrimitiveType::I32),
                        },
                    },
                    PythonFunction {
                        python_name: "echo_bool".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_bool".to_string(),
                            ffi_symbol: "boltffi_echo_bool".to_string(),
                            parameters: vec![PythonParameter {
                                name: "value".to_string(),
                                type_ref: PythonType::Primitive(PrimitiveType::Bool),
                            }],
                            return_type: PythonType::Primitive(PrimitiveType::Bool),
                        },
                    },
                    PythonFunction {
                        python_name: "echo_f32".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_f32".to_string(),
                            ffi_symbol: "boltffi_echo_f32".to_string(),
                            parameters: vec![PythonParameter {
                                name: "value".to_string(),
                                type_ref: PythonType::Primitive(PrimitiveType::F32),
                            }],
                            return_type: PythonType::Primitive(PrimitiveType::F32),
                        },
                    },
                    PythonFunction {
                        python_name: "echo_string".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_string".to_string(),
                            ffi_symbol: "boltffi_echo_string".to_string(),
                            parameters: vec![PythonParameter {
                                name: "value".to_string(),
                                type_ref: PythonType::String,
                            }],
                            return_type: PythonType::String,
                        },
                    },
                    PythonFunction {
                        python_name: "echo_bytes".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_bytes".to_string(),
                            ffi_symbol: "boltffi_echo_bytes".to_string(),
                            parameters: vec![PythonParameter {
                                name: "value".to_string(),
                                type_ref: PythonType::Sequence(PythonSequenceType::Bytes),
                            }],
                            return_type: PythonType::Sequence(PythonSequenceType::Bytes),
                        },
                    },
                    PythonFunction {
                        python_name: "echo_vec_i32".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_vec_i32".to_string(),
                            ffi_symbol: "boltffi_echo_vec_i32".to_string(),
                            parameters: vec![PythonParameter {
                                name: "values".to_string(),
                                type_ref: PythonType::Sequence(PythonSequenceType::PrimitiveVec(
                                    PrimitiveType::I32,
                                )),
                            }],
                            return_type: PythonType::Sequence(PythonSequenceType::PrimitiveVec(
                                PrimitiveType::I32,
                            )),
                        },
                    },
                    PythonFunction {
                        python_name: "echo_status".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_status".to_string(),
                            ffi_symbol: "boltffi_echo_status".to_string(),
                            parameters: vec![PythonParameter {
                                name: "value".to_string(),
                                type_ref: PythonType::CStyleEnum(PythonEnumType {
                                    native_name_stem: "status".to_string(),
                                    class_name: "Status".to_string(),
                                    tag_type: PrimitiveType::I32,
                                }),
                            }],
                            return_type: PythonType::CStyleEnum(PythonEnumType {
                                native_name_stem: "status".to_string(),
                                class_name: "Status".to_string(),
                                tag_type: PrimitiveType::I32,
                            }),
                        },
                    },
                    PythonFunction {
                        python_name: "echo_vec_status".to_string(),
                        callable: PythonCallable {
                            native_name: "echo_vec_status".to_string(),
                            ffi_symbol: "boltffi_echo_vec_status".to_string(),
                            parameters: vec![PythonParameter {
                                name: "values".to_string(),
                                type_ref: PythonType::Sequence(PythonSequenceType::CStyleEnumVec(
                                    PythonEnumType {
                                        native_name_stem: "status".to_string(),
                                        class_name: "Status".to_string(),
                                        tag_type: PrimitiveType::I32,
                                    },
                                )),
                            }],
                            return_type: PythonType::Sequence(PythonSequenceType::CStyleEnumVec(
                                PythonEnumType {
                                    native_name_stem: "status".to_string(),
                                    class_name: "Status".to_string(),
                                    tag_type: PrimitiveType::I32,
                                },
                            )),
                        },
                    },
                ],
            }
        }

        fn rendered() -> PythonPackageSources {
            PythonEmitter::emit(&Self::module())
        }

        fn rendered_file<'a>(rendered: &'a PythonPackageSources, relative_path: &str) -> &'a str {
            rendered
                .files
                .iter()
                .find(|file| file.relative_path == Path::new(relative_path))
                .map(|file| file.contents.as_str())
                .expect("expected generated file")
        }
    }

    #[test]
    fn emits_native_scalar_string_sequence_and_enum_python_package_sources() {
        let rendered = NativePythonPackageFixture::rendered();
        let pyproject_source =
            NativePythonPackageFixture::rendered_file(&rendered, "pyproject.toml");
        let setup_source = NativePythonPackageFixture::rendered_file(&rendered, "setup.py");
        let init_source =
            NativePythonPackageFixture::rendered_file(&rendered, "demo_lib/__init__.py");
        let generated_stub =
            NativePythonPackageFixture::rendered_file(&rendered, "demo_lib/__init__.pyi");
        let native_source =
            NativePythonPackageFixture::rendered_file(&rendered, "demo_lib/_native.c");

        assert!(pyproject_source.contains("build-backend = \"setuptools.build_meta\""));
        assert!(pyproject_source.ends_with('\n'));
        assert!(setup_source.contains("Extension("));
        assert!(setup_source.contains("\"demo_lib._native\""));
        assert!(setup_source.contains("\"*.pyi\""));
        assert!(init_source.contains("from enum import IntEnum"));
        assert!(init_source.contains("from pathlib import Path"));
        assert!(init_source.contains("from . import _native"));
        assert!(init_source.contains("_native._initialize_loader"));
        assert!(init_source.contains("class Status(IntEnum):"));
        assert!(init_source.contains("ACTIVE = 0"));
        assert!(init_source.contains("_native._register_status(Status)"));
        assert!(init_source.contains("class Direction(IntEnum):"));
        assert!(init_source.contains("def opposite(self) -> Direction:"));
        assert!(init_source.contains("return _native._boltffi_direction_opposite(self)"));
        assert!(init_source.contains("echo_string = _native.echo_string"));
        assert!(init_source.contains("PACKAGE_NAME = \"demo-lib\""));
        assert!(init_source.contains("\"Status\""));
        assert!(
            native_source
                .contains("typedef int32_t (*boltffi_python_symbol_echo_i32_fn)(int32_t);")
        );
        assert!(native_source.contains(
            "typedef FfiBuf_u8 (*boltffi_python_symbol_echo_string_fn)(const uint8_t *, uintptr_t);"
        ));
        assert!(native_source.contains(
            "typedef FfiBuf_u8 (*boltffi_python_symbol_echo_bytes_fn)(const uint8_t *, uintptr_t);"
        ));
        assert!(native_source.contains(
            "typedef FfiBuf_u8 (*boltffi_python_symbol_echo_vec_i32_fn)(const int32_t *, uintptr_t);"
        ));
        assert!(
            native_source
                .contains("typedef int32_t (*boltffi_python_symbol_echo_status_fn)(int32_t);")
        );
        assert!(native_source.contains(
            "typedef FfiBuf_u8 (*boltffi_python_symbol_echo_vec_status_fn)(const uint8_t *, uintptr_t);"
        ));
        assert!(native_source.contains("static PyObject *boltffi_python_initialize_loader"));
        assert!(native_source.contains("static int boltffi_python_parse_i32"));
        assert!(native_source.contains("static int boltffi_python_parse_status"));
        assert!(native_source.contains("static PyObject *boltffi_python_box_status"));
        assert!(native_source.contains("if (boxed_value == NULL) {"));
        assert!(native_source.contains("static int boltffi_python_status_native_to_wire_tag"));
        assert!(native_source.contains("static PyObject *boltffi_python_box_status_from_wire_tag"));
        assert!(native_source.contains("static int boltffi_python_parse_vec_status"));
        assert!(native_source.contains("static PyObject *boltffi_python_decode_owned_vec_status"));
        assert!(native_source.contains("static PyObject *boltffi_python_wrapper_register_status"));
        assert!(native_source.contains("static int boltffi_python_parse_string"));
        assert!(native_source.contains("static int boltffi_python_parse_bytes"));
        assert!(native_source.contains("static int boltffi_python_parse_vec_i32"));
        assert!(
            native_source.contains("static PyObject *boltffi_python_callable_wrapper_echo_bool")
        );
        assert!(native_source.contains("static PyObject *boltffi_python_decode_owned_utf8"));
        assert!(native_source.contains("static PyObject *boltffi_python_decode_owned_bytes"));
        assert!(native_source.contains("static PyObject *boltffi_python_decode_owned_vec_i32"));
        assert!(native_source.contains("boltffi_python_free_buf_symbol"));
        assert!(native_source.contains("boltffi_python_buffer_input"));
        assert!(native_source.contains("boltffi_python_release_buffer_input"));
        assert!(native_source.contains("PyUnicode_AsUTF8AndSize"));
        assert!(native_source.contains("PyUnicode_DecodeUTF8"));
        assert!(native_source.contains("wchar_t *wide_library_path = NULL;"));
        assert!(native_source.contains("dlsym"));
        assert!(native_source.contains("GetProcAddress"));
        assert!(native_source.contains("FLT_MAX"));
        assert!(!native_source.contains("isfinite"));
        assert!(native_source.contains("METH_FASTCALL"));
        assert!(pyproject_source.contains("wheel>=0.43"));
        assert!(generated_stub.contains("class Status(IntEnum):"));
        assert!(generated_stub.contains("def echo_status"));
        assert!(generated_stub.contains("def opposite(self) -> Direction:"));
    }
}
