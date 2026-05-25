use boltffi_ffi_rules::primitive::Primitive;
use syn::Type;

use super::callback_return::try_resolve_callback_handle_return;
use crate::index::data_types::DataTypeCategory;
use crate::lowering::transport::{
    NamedTypeTransport, RustTypeShape, StandardContainer, TypeDescriptor,
};

use super::model::{
    EncodedReturnStrategy, ReturnLoweringContext, ScalarReturnStrategy, ValueReturnStrategy,
};

pub(super) fn option_primitive_uses_scalar_encoding(primitive: Primitive) -> bool {
    !matches!(primitive, Primitive::I64 | Primitive::U64)
}

#[derive(Clone, Copy)]
pub struct ReturnTypeDescriptor<'a> {
    type_descriptor: TypeDescriptor<'a>,
}

impl<'a> ReturnTypeDescriptor<'a> {
    pub fn parse(rust_type: &'a Type) -> Self {
        Self {
            type_descriptor: TypeDescriptor::new(rust_type),
        }
    }

    pub fn option_primitive(&self) -> Option<Primitive> {
        match self.type_descriptor.standard_container() {
            Some(StandardContainer::Option(inner_type)) => {
                TypeDescriptor::new(inner_type).primitive()
            }
            _ => None,
        }
    }

    pub fn option_encoded_return_strategy(&self) -> EncodedReturnStrategy {
        let Some(primitive) = self.option_primitive() else {
            return EncodedReturnStrategy::WireEncoded;
        };

        if option_primitive_uses_scalar_encoding(primitive) {
            EncodedReturnStrategy::OptionScalar
        } else {
            EncodedReturnStrategy::WireEncoded
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self.type_descriptor.shape(),
            RustTypeShape::Unit | RustTypeShape::Primitive(_)
        )
    }
}

pub fn classify_value_return_strategy(
    rust_type: &Type,
    return_lowering: &ReturnLoweringContext<'_>,
) -> syn::Result<ValueReturnStrategy> {
    let return_type = ReturnTypeDescriptor::parse(rust_type);

    match return_type.type_descriptor.shape() {
        RustTypeShape::Unit => Ok(ValueReturnStrategy::Void),
        RustTypeShape::Utf8String => Ok(ValueReturnStrategy::Buffer(
            EncodedReturnStrategy::Utf8String,
        )),
        RustTypeShape::Primitive(_) => Ok(ValueReturnStrategy::Scalar(
            ScalarReturnStrategy::PrimitiveValue,
        )),
        RustTypeShape::StandardContainer(StandardContainer::Vec(inner_type)) => {
            let buffer_strategy = if return_lowering
                .named_type_transport_classifier()
                .supports_direct_vec_transport(inner_type)
            {
                EncodedReturnStrategy::DirectVec
            } else {
                EncodedReturnStrategy::WireEncoded
            };
            Ok(ValueReturnStrategy::Buffer(buffer_strategy))
        }
        RustTypeShape::StandardContainer(StandardContainer::Result { ok, err }) => {
            if ReturnTypeDescriptor::parse(ok).is_primitive()
                && ReturnTypeDescriptor::parse(err).is_primitive()
            {
                Ok(ValueReturnStrategy::Buffer(EncodedReturnStrategy::ResultScalar))
            } else {
                Ok(ValueReturnStrategy::Buffer(EncodedReturnStrategy::WireEncoded))
            }
        }
        RustTypeShape::StandardContainer(StandardContainer::Option(inner_type)) => {
            if ReturnTypeDescriptor::parse(inner_type).is_primitive() {
                Ok(ValueReturnStrategy::Buffer(
                    return_type.option_encoded_return_strategy(),
                ))
            } else if return_lowering
                .exported_classes()
                .is_exported_class_type(inner_type)
            {
                Ok(ValueReturnStrategy::NullableObjectHandle)
            } else if return_lowering
                .exported_classes()
                .is_boxed_trait_wrapped_by_exported_handle(inner_type)
            {
                Ok(ValueReturnStrategy::NullableObjectHandle)
            } else {
                Ok(ValueReturnStrategy::Buffer(EncodedReturnStrategy::WireEncoded))
            }
        }
        RustTypeShape::NamedNominal | RustTypeShape::GenericNominal | RustTypeShape::Other => {
            // `Box<dyn Trait>` with `TraitHandle` exported must classify as object handle before
            // callback resolution — async callback traits reject `supports_local_handle` with an error.
            if return_lowering
                .exported_classes()
                .is_boxed_trait_wrapped_by_exported_handle(rust_type)
            {
                return Ok(ValueReturnStrategy::ObjectHandle);
            }

            if try_resolve_callback_handle_return(rust_type, return_lowering.callback_traits())?
                .is_some()
            {
                return Ok(ValueReturnStrategy::CallbackHandle);
            }

            if return_lowering
                .exported_classes()
                .is_exported_class_type(rust_type)
            {
                return Ok(ValueReturnStrategy::ObjectHandle);
            }

            match return_lowering
                .named_type_transport_classifier()
                .classify_named_type_transport(rust_type)
            {
                NamedTypeTransport::WireEncoded => Ok(ValueReturnStrategy::Buffer(
                    EncodedReturnStrategy::WireEncoded,
                )),
                NamedTypeTransport::ExportedClass => Ok(ValueReturnStrategy::ObjectHandle),
                NamedTypeTransport::Passable => {
                    match return_lowering.data_types().category_for(rust_type) {
                        Some(DataTypeCategory::Scalar) => Ok(ValueReturnStrategy::Scalar(
                            ScalarReturnStrategy::CStyleEnumTag,
                        )),
                        Some(DataTypeCategory::Blittable) => Ok(ValueReturnStrategy::CompositeValue),
                        Some(DataTypeCategory::WireEncoded) | None => unreachable!(
                            "passable return transport requires scalar or blittable data type"
                        ),
                    }
                }
            }
        }
    }
}

/// inner `T` for `Option<T>` (used for nullable exported-class params/returns).
pub(crate) fn option_inner_type(ty: &Type) -> Option<Type> {
    use crate::lowering::transport::{StandardContainer, TypeDescriptor};
    match TypeDescriptor::new(ty).standard_container() {
        Some(StandardContainer::Option(inner)) => Some(inner.clone()),
        _ => None,
    }
}
