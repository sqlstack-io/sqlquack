//! Field types.

use syn::Field;
use syn::GenericArgument;
use syn::PathArguments;
use syn::Type;

use crate::constants::SKIP_MARKER;

pub(crate) fn container_inner<'a>(ty: &'a Type, expected: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != expected {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

pub(crate) fn is_expression(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "Expression"))
}

pub(crate) fn is_scalar(ty: &Type) -> bool {
    let Type::Path(path) = ty else { return false };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    matches!(
        segment.ident.to_string().as_str(),
        "bool"
            | "char"
            | "str"
            | "String"
            | "usize"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "f32"
            | "f64"
            | "Span"
    )
}

pub(crate) fn is_skipped(field: &Field) -> bool {
    field.attrs.iter().any(|attribute| {
        attribute.path().is_ident("ast")
            && attribute
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == SKIP_MARKER)
    })
}
