use boltffi_ffi_rules::callable::ExecutionKind;

use crate::ir::definitions::{FunctionDef, ParamDef, ParamPassing, ReturnDef};
use crate::ir::ids::{FunctionId, ParamName};
use crate::ir::types::{PrimitiveType, TypeExpr};
use crate::ir::{AbiContract, FfiContract, Lowerer, PackageInfo, TypeCatalog};
use crate::render::python::{PythonLowerError, PythonModule};

pub(super) fn test_function(function_name: &str, parameter_names: &[&str]) -> FunctionDef {
    FunctionDef {
        id: FunctionId::new(function_name),
        params: parameter_names
            .iter()
            .map(|parameter_name| ParamDef {
                name: ParamName::new(*parameter_name),
                type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                passing: ParamPassing::Value,
                doc: None,
            })
            .collect(),
        returns: ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::I32)),
        execution_kind: ExecutionKind::Sync,
        doc: None,
        deprecated: None,
    }
}

pub(super) fn lower_contract(
    catalog: TypeCatalog,
    functions: Vec<FunctionDef>,
) -> Result<PythonModule, PythonLowerError> {
    let ffi_contract = FfiContract {
        package: PackageInfo {
            name: "demo".to_string(),
            version: Some("0.1.0".to_string()),
        },
        catalog,
        functions,
    };
    let abi_contract = Lowerer::new(&ffi_contract).to_abi_contract();

    super::PythonLowerer::new(
        &ffi_contract,
        &abi_contract,
        "demo",
        "demo",
        Some("0.1.0".to_string()),
        "demo",
    )
    .lower()
}

pub(super) fn contract_parts(
    catalog: TypeCatalog,
    functions: Vec<FunctionDef>,
) -> (FfiContract, AbiContract) {
    let ffi_contract = FfiContract {
        package: PackageInfo {
            name: "demo".to_string(),
            version: Some("0.1.0".to_string()),
        },
        catalog,
        functions,
    };
    let abi_contract = Lowerer::new(&ffi_contract).to_abi_contract();

    (ffi_contract, abi_contract)
}
