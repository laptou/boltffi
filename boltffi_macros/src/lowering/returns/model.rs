pub use boltffi_ffi_rules::transport::{
    DirectBufferReturnMethod, EncodedReturnStrategy, ErrorReturnStrategy, ReturnContract,
    ReturnInvocationContext, ReturnPlatform, ScalarReturnStrategy, ValueReturnMethod,
    ValueReturnStrategy,
};
use syn::visit_mut::{self, VisitMut};
use syn::{ReturnType, Type};

use crate::index::callback_traits::CallbackTraitRegistry;
use crate::index::custom_types::CustomTypeRegistry;
use crate::index::data_types::DataTypeRegistry;
use crate::index::exported_classes::ExportedClassRegistry;
use crate::lowering::transport::{NamedTypeTransportClassifier, StandardContainer, TypeDescriptor};

use super::classify::{ReturnTypeDescriptor, classify_value_return_strategy};

#[derive(Clone)]
pub struct ResolvedReturn {
    rust_type: syn::Type,
    return_contract: ReturnContract,
}

impl ResolvedReturn {
    pub fn new(rust_type: syn::Type, return_contract: ReturnContract) -> Self {
        Self {
            rust_type,
            return_contract,
        }
    }

    pub fn rust_type(&self) -> &syn::Type {
        &self.rust_type
    }

    pub fn value_return_strategy(&self) -> ValueReturnStrategy {
        self.return_contract.value_strategy()
    }

    pub fn encoded_return_strategy(&self) -> Option<EncodedReturnStrategy> {
        match self.return_contract.value_strategy() {
            ValueReturnStrategy::Buffer(strategy) => Some(strategy),
            _ => None,
        }
    }

    pub fn is_unit(&self) -> bool {
        matches!(
            self.return_contract.value_strategy(),
            ValueReturnStrategy::Void
        )
    }

    pub fn is_primitive_scalar(&self) -> bool {
        matches!(
            self.return_contract.value_strategy(),
            ValueReturnStrategy::Scalar(ScalarReturnStrategy::PrimitiveValue)
        )
    }

    pub fn is_passable_value(&self) -> bool {
        matches!(
            self.return_contract.value_strategy(),
            ValueReturnStrategy::Scalar(ScalarReturnStrategy::CStyleEnumTag)
                | ValueReturnStrategy::CompositeValue
        )
    }

    pub fn value_return_method(
        &self,
        context: ReturnInvocationContext,
        platform: ReturnPlatform,
    ) -> ValueReturnMethod {
        self.return_contract.value_return_method(context, platform)
    }

    pub fn direct_buffer_return_method(
        &self,
        context: ReturnInvocationContext,
        platform: ReturnPlatform,
    ) -> Option<DirectBufferReturnMethod> {
        self.return_contract
            .direct_buffer_return_method(context, platform)
    }

    pub fn error_strategy(&self) -> ErrorReturnStrategy {
        self.return_contract.error_strategy()
    }

    /// For `Result<OkT, ErrT>` lowered as a fallible handle return, the success value type.
    pub fn fallible_ok_type(&self) -> Option<Type> {
        if self.error_strategy() != ErrorReturnStrategy::Encoded {
            return None;
        }
        match TypeDescriptor::new(self.rust_type()).standard_container() {
            Some(StandardContainer::Result { ok, .. }) => Some(ok.clone()),
            _ => None,
        }
    }

    /// async `complete` adds `err_out_ptr` / `err_out_len` for typed `Err` (native + wasm).
    pub fn async_complete_needs_err_carrier(&self) -> bool {
        matches!(
            self.value_return_strategy(),
            ValueReturnStrategy::ObjectHandle | ValueReturnStrategy::CallbackHandle
        ) && self.error_strategy() == ErrorReturnStrategy::Encoded
            && self.fallible_ok_type().is_some()
    }
}

#[derive(Clone, Copy)]
pub struct ReturnLoweringContext<'a> {
    custom_types: &'a CustomTypeRegistry,
    data_types: &'a DataTypeRegistry,
    exported_classes: &'a ExportedClassRegistry,
    callback_traits: &'a CallbackTraitRegistry,
}

impl<'a> ReturnLoweringContext<'a> {
    pub fn new(
        custom_types: &'a CustomTypeRegistry,
        data_types: &'a DataTypeRegistry,
        exported_classes: &'a ExportedClassRegistry,
        callback_traits: &'a CallbackTraitRegistry,
    ) -> Self {
        Self {
            custom_types,
            data_types,
            exported_classes,
            callback_traits,
        }
    }

    pub fn custom_types(&self) -> &'a CustomTypeRegistry {
        self.custom_types
    }

    pub fn data_types(&self) -> &'a DataTypeRegistry {
        self.data_types
    }

    pub fn exported_classes(&self) -> &'a ExportedClassRegistry {
        self.exported_classes
    }

    pub fn callback_traits(&self) -> &'a CallbackTraitRegistry {
        self.callback_traits
    }

    pub(crate) fn named_type_transport_classifier(&self) -> NamedTypeTransportClassifier<'a> {
        NamedTypeTransportClassifier::new(self.custom_types, self.data_types, self.exported_classes)
    }

    pub fn lower_output(&self, output: &ReturnType) -> syn::Result<ResolvedReturn> {
        match output {
            ReturnType::Default => Ok(ResolvedReturn::new(
                syn::parse_quote!(()),
                ReturnContract::infallible(ValueReturnStrategy::Void),
            )),
            ReturnType::Type(_, rust_type) => self.lower_return_type(rust_type),
        }
    }

    /// Return-type classification (exports, callback returns). Handles `Result` + handle `Ok` specially.
    pub fn lower_return_type(&self, rust_type: &Type) -> syn::Result<ResolvedReturn> {
        if let Some(StandardContainer::Result { ok, err }) =
            TypeDescriptor::new(rust_type).standard_container()
        {
            if ReturnTypeDescriptor::parse(ok).is_primitive()
                && ReturnTypeDescriptor::parse(err).is_primitive()
            {
                return Ok(ResolvedReturn::new(
                    rust_type.clone(),
                    ReturnContract::infallible(ValueReturnStrategy::Buffer(
                        EncodedReturnStrategy::ResultScalar,
                    )),
                ));
            }

            let ok_strategy = classify_value_return_strategy(ok, self)?;
            if matches!(
                ok_strategy,
                ValueReturnStrategy::ObjectHandle | ValueReturnStrategy::CallbackHandle
            ) {
                return Ok(ResolvedReturn::new(
                    rust_type.clone(),
                    ReturnContract::new(ok_strategy, ErrorReturnStrategy::Encoded),
                ));
            }

            return Ok(ResolvedReturn::new(
                rust_type.clone(),
                ReturnContract::infallible(ValueReturnStrategy::Buffer(
                    EncodedReturnStrategy::WireEncoded,
                )),
            ));
        }

        Ok(ResolvedReturn::new(
            rust_type.clone(),
            ReturnContract::new(
                classify_value_return_strategy(rust_type, self)?,
                ErrorReturnStrategy::None,
            ),
        ))
    }

    /// Params and non-return contexts: keep wire `Result` classification from [`classify_value_return_strategy`].
    pub fn lower_type(&self, rust_type: &Type) -> syn::Result<ResolvedReturn> {
        Ok(ResolvedReturn::new(
            rust_type.clone(),
            ReturnContract::new(
                classify_value_return_strategy(rust_type, self)?,
                ErrorReturnStrategy::None,
            ),
        ))
    }
}

/// replace `Self` in a type tree with the concrete impl type so lowered exports can use the type
/// outside the `impl` block (e.g. `rust_future_poll::<Result<T, E>>`).
pub fn replace_self_in_type(ty: &Type, concrete: &syn::Ident) -> Type {
    let mut ty = ty.clone();
    ReplaceSelfWithType { concrete }.visit_type_mut(&mut ty);
    ty
}

pub fn normalize_return_type_for_self(output: &ReturnType, concrete: &syn::Ident) -> ReturnType {
    match output {
        ReturnType::Default => ReturnType::Default,
        ReturnType::Type(arrow, ty) => ReturnType::Type(
            *arrow,
            Box::new(replace_self_in_type(ty.as_ref(), concrete)),
        ),
    }
}

struct ReplaceSelfWithType<'a> {
    concrete: &'a syn::Ident,
}

impl VisitMut for ReplaceSelfWithType<'_> {
    fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
        if i.qself.is_none() && i.path.is_ident("Self") {
            *i = syn::TypePath {
                qself: None,
                path: syn::Path::from(self.concrete.clone()),
            };
        } else {
            visit_mut::visit_type_path_mut(self, i);
        }
    }
}

#[derive(Clone, Copy)]
pub struct WasmOptionScalarEncoding {
    pub(super) primitive: boltffi_ffi_rules::primitive::Primitive,
}
