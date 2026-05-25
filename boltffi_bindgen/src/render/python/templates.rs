use askama::Template;

use crate::ir::types::PrimitiveType;
use crate::render::python::PythonModule;
use crate::render::python::primitives::{
    CPythonCallableExt as _, CPythonParameterExt as _, CPythonPrimitiveTypeExt as _,
    CPythonTypeExt as _,
};

#[derive(Template)]
#[template(path = "render_python/init_py.txt", escape = "none")]
pub struct InitTemplate<'a> {
    pub module: &'a PythonModule,
}

#[derive(Template)]
#[template(path = "render_python/init_pyi.txt", escape = "none")]
pub struct InitStubTemplate<'a> {
    pub module: &'a PythonModule,
}

#[derive(Template)]
#[template(path = "render_python/pyproject.toml.txt", escape = "none")]
pub struct PyprojectTemplate;

#[derive(Template)]
#[template(path = "render_python/setup.py.txt", escape = "none")]
pub struct SetupTemplate<'a> {
    pub module: &'a PythonModule,
    pub package_version_literal: &'a str,
    pub minimum_python_version_requirement_literal: &'a str,
    pub native_extension_name_literal: &'a str,
    pub native_source_path_literal: &'a str,
}

#[derive(Template)]
#[template(path = "render_python/native_module.c.txt", escape = "none")]
pub struct NativeModuleTemplate<'a> {
    pub module: &'a PythonModule,
    pub used_primitive_types: &'a [PrimitiveType],
}
