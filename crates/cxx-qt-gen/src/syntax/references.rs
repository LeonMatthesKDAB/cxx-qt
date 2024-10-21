// SPDX-FileCopyrightText: 2024 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Leon Matthes <leon.matthes@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_quote_spanned, spanned::Spanned, Result, Type};

pub fn to_pointer_type(ty: &Type) -> Result<Type> {
    match ty {
        Type::Reference(reference) => {
            let ty = reference.elem.clone();

            let mutability = if let Some(mutability) = &reference.mutability {
                quote_spanned! {mutability.span() => mut}
            } else {
                quote_spanned! {reference.span() => const }
            };

            Ok(parse_quote_spanned! {
                ty.span() => *#mutability #ty
            })
        }
        other => Ok(other.clone()),
    }
}

pub fn convert_to_pointer(ty: &Type, expr: TokenStream) -> Result<TokenStream> {
    match ty {
        Type::Reference(reference) => {
            let pointer_type = to_pointer_type(ty)?;

            Ok(quote_spanned! {
                reference.span() => #expr as #pointer_type
            })
        }
        _ => Ok(expr),
    }
}

pub fn convert_from_pointer(target_ty: &Type, expr: TokenStream) -> Result<TokenStream> {
    match target_ty {
        Type::Reference(reference) => {
            let mutability = if let Some(mutability) = &reference.mutability {
                mutability.into_token_stream()
            } else {
                quote! {}
            };

            Ok(quote_spanned! {
                reference.span() => unsafe { &#mutability *#expr }
            })
        }
        _ => Ok(expr),
    }
}

#[cfg(test)]
mod tests {
    use super::to_pointer_type;
    use quote::ToTokens;
    use syn::parse_quote;

    use crate::tests::assert_tokens_eq;

    macro_rules! assert_to_pointer {
        ($({$($input:tt)*} => {$($output:tt)*})*) => {
            $({
                assert_tokens_eq(
                    &to_pointer_type(&parse_quote! {$($input)*}).unwrap(),
                    parse_quote! {$($output)*},
                );
            })*
        };
    }

    #[test]
    fn test_to_pointer_type() {
        assert_to_pointer!(
            {&i32} => {*const i32}
            {&mut i32} => {*mut i32}
            {&'a i32} => {*const i32}
            {&'a mut i32} => {*mut i32}

            {my::Type} => {my::Type}
            {*const i32} => {*const i32}
            {*mut i32} => {*mut i32}
        );
    }

    macro_rules! assert_convert_to_pointer {
        ($(($ty:ty, $($expr:tt)*) => {$($output:tt)*})*) => {
            $({
                assert_tokens_eq(
                    &super::convert_to_pointer(&parse_quote! {$ty}, quote::quote! {$($expr)*}).unwrap(),
                    parse_quote! {$($output)*},
                );
            })*
        };
    }

    #[test]
    fn test_convert_to_pointer() {
        assert_convert_to_pointer! {
            (&'a i32, 5) => {5 as *const i32}
            (&i32, a.b) => { a.b as *const i32}
            (&mut i32, a.b) => { a.b as *mut i32}
            (&'_ mut i32, a.b) => { a.b as *mut i32}

            (i32, 5) => {5}
            (i32, a.b) => {a.b}
            (T, a.b) => {a.b}
            ((T, X, Y), a.b) => {a.b}
        }
    }

    macro_rules! assert_convert_from_pointer {
        ($(($ty:ty, $($expr:tt)*) => {$($output:tt)*})*) => {
            $({
                assert_tokens_eq(
                    &super::convert_from_pointer(&parse_quote! {$ty}, quote::quote! {$($expr)*}).unwrap(),
                    parse_quote! {$($output)*},
                );
            })*
        };
    }

    #[test]
    fn test_convert_from_pointer() {
        assert_convert_from_pointer! {
            (&'a i32, 5) => {unsafe { &*5 }}
            (&i32, 5) => {unsafe {&*5}}
            (&'a mut i32, 5) => {unsafe {&mut *5}}
            (&mut i32, 5) => {unsafe {&mut *5}}

            (T, a.b) => {a.b}
        }
    }
}
