// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use proc_macro2::Span;
use syn::{ForeignItemFn, Ident};

use crate::{
    generator::naming::{property::QPropertyName, qobject::QObjectNames, CombinedIdent},
    parser::signals::ParsedSignal,
};

pub fn generate(idents: &QPropertyName, qobject_idents: &QObjectNames) -> ParsedSignal {
    // We build our signal in the generation phase as we need to use the naming
    // structs to build the signal name
    let cpp_class_rust = &qobject_idents.name.rust_unqualified();
    let notify_cpp = &idents.notify.cxx_unqualified();
    let notify_rust = idents.notify.rust_unqualified();
    let method: ForeignItemFn = syn::parse_quote! {
        #[doc = "Notify for the Q_PROPERTY"]
        #[cxx_name = #notify_cpp]
        fn #notify_rust(self: Pin<&mut #cpp_class_rust>);
    };
    ParsedSignal::from_property_method(
        method,
        CombinedIdent {
            cpp: Ident::new(&idents.notify.cxx_unqualified(), Span::call_site()),
            rust: idents.notify.rust_unqualified().clone(),
        },
        qobject_idents.name.rust_unqualified().clone(),
    )
}
