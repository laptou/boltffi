pub(crate) mod callback_return;
pub(crate) mod classify;
pub(crate) mod lower;
pub(crate) mod model;

#[cfg(test)]
mod tests {
    use super::model::{ResolvedReturn, WasmOptionScalarEncoding};
    use boltffi_ffi_rules::transport::{
        EncodedReturnStrategy, ErrorReturnStrategy, ReturnContract, ReturnInvocationContext,
        ReturnPlatform, ValueReturnMethod, ValueReturnStrategy,
    };
    use syn::parse_quote;

    #[test]
    fn wasm_option_bool_uses_numeric_bool_encoding() {
        let value_ident = syn::Ident::new("value", proc_macro2::Span::call_site());
        let expression =
            WasmOptionScalarEncoding::from_option_rust_type(&parse_quote!(Option<bool>))
                .expect("expected bool option encoding")
                .some_expression(&value_ident)
                .to_string();

        assert_eq!(expression, "if value { 1.0 } else { 0.0 }");
    }

    #[test]
    fn packed_encoded_return_uses_packed_default_on_wasm_failure() {
        let resolved_return = ResolvedReturn::new(
            parse_quote!(std::time::Duration),
            ReturnContract::infallible(ValueReturnStrategy::Buffer(
                EncodedReturnStrategy::WireEncoded,
            )),
        );

        let statement = resolved_return
            .invalid_arg_early_return_statement()
            .to_string();

        assert!(matches!(
            resolved_return.value_return_strategy(),
            ValueReturnStrategy::Buffer(EncodedReturnStrategy::WireEncoded)
        ));
        assert!(matches!(
            resolved_return
                .value_return_method(ReturnInvocationContext::SyncExport, ReturnPlatform::Wasm,),
            ValueReturnMethod::DirectReturn
        ));
        assert!(statement.contains("FfiBuf :: default () . into_packed ()"));
        assert!(statement.contains("return :: boltffi :: __private :: FfiBuf :: default ()"));
    }

    #[test]
    fn direct_vec_return_uses_void_wasm_failure() {
        let resolved_return = ResolvedReturn::new(
            parse_quote!(Vec<i32>),
            ReturnContract::infallible(ValueReturnStrategy::Buffer(
                EncodedReturnStrategy::DirectVec,
            )),
        );

        assert!(matches!(
            resolved_return
                .value_return_method(ReturnInvocationContext::SyncExport, ReturnPlatform::Wasm,),
            ValueReturnMethod::WriteToReturnSlot
        ));
        assert_eq!(
            resolved_return
                .invalid_arg_early_return_statement()
                .to_string(),
            "return ;"
        );
    }

    #[test]
    fn fallible_handle_return_contract_pairs_object_handle_with_encoded_error() {
        let resolved_return = ResolvedReturn::new(
            parse_quote!(Result<Transfer, TransferError>),
            ReturnContract::new(
                ValueReturnStrategy::ObjectHandle,
                ErrorReturnStrategy::Encoded,
            ),
        );

        assert_eq!(
            resolved_return.error_strategy(),
            ErrorReturnStrategy::Encoded
        );
        assert!(matches!(
            resolved_return.value_return_strategy(),
            ValueReturnStrategy::ObjectHandle
        ));
        let ok = resolved_return
            .fallible_ok_type()
            .expect("fallible ok type");
        assert!(::quote::quote!(#ok).to_string().contains("Transfer"));

        let stmt = resolved_return
            .invalid_arg_early_return_statement()
            .to_string();
        assert!(stmt.contains("Default :: default ()"));
    }
}
