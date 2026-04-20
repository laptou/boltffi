use crate::ir::abi::{AbiEnum, CallId};
use crate::ir::definitions::FunctionDef;
use crate::ir::ids::EnumId;

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn resolve_function_symbol(&self, function: &FunctionDef) -> Option<String> {
        self.abi_contract
            .calls
            .iter()
            .find(|call| matches!(&call.id, CallId::Function(function_id) if *function_id == function.id))
            .map(|call| call.symbol.as_str().to_string())
    }

    pub(super) fn resolve_call_symbol(&self, call_id: &CallId) -> String {
        self.abi_contract
            .calls
            .iter()
            .find(|call| call.id == *call_id)
            .map(|call| call.symbol.as_str().to_string())
            .unwrap_or_else(|| panic!("python lowering missing ABI call for {:?}", call_id))
    }

    pub(super) fn resolve_abi_enum(&self, enum_id: &EnumId) -> &AbiEnum {
        self.abi_contract
            .enums
            .iter()
            .find(|abi_enum| abi_enum.id == *enum_id)
            .unwrap_or_else(|| panic!("python lowering missing ABI enum for {:?}", enum_id))
    }
}
