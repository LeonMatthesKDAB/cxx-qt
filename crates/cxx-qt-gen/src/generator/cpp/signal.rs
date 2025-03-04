// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    generator::{
        cfg::try_eval_attributes,
        cpp::{fragment::CppFragment, qobject::GeneratedCppQObjectBlocks},
        naming::{
            qobject::QObjectNames,
            signals::{QSignalHelperNames, QSignalNames},
        },
        GeneratedOpt,
    },
    naming::{cpp::syn_type_to_cpp_type, Name, TypeNames},
    parser::{parameter::ParsedFunctionParameter, signals::ParsedSignal},
};
use indoc::formatdoc;
use std::collections::BTreeSet;
use syn::Result;

#[derive(Default)]
pub struct CppSignalFragment {
    /// List of includes
    pub includes: BTreeSet<String>,
    /// List of forward declares that go before the CXX include
    pub forward_declares: Vec<String>,
    /// List of fragments that go at the top of the header or source
    ///
    /// Note that these should include their namespace
    pub fragments: Vec<CppFragment>,
    /// Any methods for the class
    pub methods: Vec<CppFragment>,
}

/// Combined output of possible parameter lines to be used
struct Parameters {
    /// name with type of parameters
    named_types: String,
    /// name with type of parameters including self
    named_types_with_self: String,
    /// Raw types of the parameters including self
    types_with_self: String,
    /// Raw ::std::move values of the parameters including self
    values_with_self: String,
}

/// From given parameters, mappings, and self value constructor the combined parameter lines
fn parameter_types_and_values(
    parameters: &[ParsedFunctionParameter],
    type_names: &TypeNames,
    self_ty: &Name,
) -> Result<Parameters> {
    let mut parameter_named_types_with_self = vec![];
    let mut parameter_types_with_self = vec![];
    let mut parameter_values_with_self = vec![];

    for parameter in parameters {
        let cxx_ty = syn_type_to_cpp_type(&parameter.ty, type_names)?;
        let ident_str = parameter.ident.to_string();
        parameter_named_types_with_self.push(format!("{cxx_ty} {ident_str}",));
        parameter_types_with_self.push(cxx_ty.clone());
        parameter_values_with_self.push(format!("::std::move({ident_str})"));
    }

    let parameter_named_types = parameter_named_types_with_self.join(", ");

    // Insert the extra argument into the closure
    let self_ty = self_ty.cxx_qualified();
    parameter_named_types_with_self.insert(0, format!("{self_ty}& self"));
    parameter_types_with_self.insert(0, format!("{self_ty}&"));
    parameter_values_with_self.insert(0, "self".to_owned());

    Ok(Parameters {
        named_types: parameter_named_types,
        named_types_with_self: parameter_named_types_with_self.join(", "),
        types_with_self: parameter_types_with_self.join(", "),
        values_with_self: parameter_values_with_self.join(", "),
    })
}

pub fn generate_cpp_signal(
    signal: &ParsedSignal,
    qobject_name: &Name,
    type_names: &TypeNames,
    opt: &GeneratedOpt,
) -> Result<CppSignalFragment> {
    let mut generated = CppSignalFragment::default();

    // Skip if the cfg attributes are not resolved to true
    if !try_eval_attributes(opt.cfg_evaluator.as_ref(), &signal.cfgs)? {
        return Ok(generated);
    }

    // Add the include we need
    generated
        .includes
        .insert("#include <cxx-qt/signalhandler.h>".to_owned());

    // Build a namespace that includes any namespace for the T
    let qobject_ident_namespaced = qobject_name.cxx_qualified();

    // Prepare the idents
    let idents = QSignalNames::from(signal);
    let idents_helper = QSignalHelperNames::new(&idents, qobject_name)?;

    let signal_ident = idents.name.cxx_unqualified();
    let free_connect_ident_cpp = idents_helper.connect_name.cxx_unqualified();

    // Retrieve the parameters for the signal
    let parameters = parameter_types_and_values(&signal.parameters, type_names, qobject_name)?;
    let parameters_named_types = parameters.named_types;
    let parameters_named_types_with_self = parameters.named_types_with_self;
    let parameter_types_with_self = parameters.types_with_self;
    let parameter_values_with_self = parameters.values_with_self;

    let param_struct = idents_helper.struct_param;
    let signal_handler_alias = idents_helper.handler_alias;
    let signal_handler_alias_namespaced = idents_helper.handler_alias_namespaced;
    let signal_handler_call = idents_helper.function_call;
    let signal_handler_drop = idents_helper.function_drop;
    let namespace = idents_helper.namespace;

    let signal_handler_type = format!("SignalHandler<::{namespace}::{param_struct} *>");

    generated.forward_declares.push(formatdoc! {
        r#"
        namespace {namespace} {{
        using {signal_handler_alias} = ::rust::cxxqt1::SignalHandler<struct {param_struct} *>;
        }} // namespace {namespace}
        "#
    });

    // Generate the Q_SIGNAL if this is not an existing signal
    if !signal.inherit {
        generated.methods.push(CppFragment::Header(format!(
            "Q_SIGNAL void {signal_ident}({parameters_named_types});"
        )));
    }

    generated.fragments.push(CppFragment::Pair {
        header: formatdoc! {
        r#"
            namespace {namespace} {{
            ::QMetaObject::Connection
            {free_connect_ident_cpp}({qobject_ident_namespaced}& self, {signal_handler_alias_namespaced} closure, ::Qt::ConnectionType type);
            }} // namespace {namespace}
            "#
        },
        source: formatdoc! {
            r#"
            // Define namespace otherwise we hit a GCC bug
            // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56480
            namespace rust::cxxqt1 {{
            template <>
            {signal_handler_type}::~SignalHandler() noexcept
            {{
                if (data[0] == nullptr && data[1] == nullptr)
                {{
                    return;
                }}

                {signal_handler_drop}(::std::move(*this));
            }}

            template <>
            template <>
            void {signal_handler_type}::operator()<{parameter_types_with_self}>({parameters_named_types_with_self})
            {{
                {signal_handler_call}(*this, {parameter_values_with_self});
            }}

            static_assert(alignof({signal_handler_type}) <= alignof(::std::size_t), "unexpected aligment");
            static_assert(sizeof({signal_handler_type}) == sizeof(::std::size_t[2]), "unexpected size");
            }} // namespace rust::cxxqt1

            namespace {namespace} {{
            ::QMetaObject::Connection
            {free_connect_ident_cpp}({qobject_ident_namespaced}& self, {signal_handler_alias_namespaced} closure, ::Qt::ConnectionType type)
            {{
                return ::QObject::connect(
                    &self,
                    &{qobject_ident_namespaced}::{signal_ident},
                    &self,
                    [&, closure = ::std::move(closure)]({parameters_named_types}) mutable {{
                        closure.template operator()<{parameter_types_with_self}>({parameter_values_with_self});
                    }},
                    type);
            }}
            }} // namespace {namespace}
        "#,
        }
    });

    Ok(generated)
}

pub fn generate_cpp_signals(
    signals: &Vec<&ParsedSignal>,
    qobject_idents: &QObjectNames,
    type_names: &TypeNames,
    opt: &GeneratedOpt,
) -> Result<GeneratedCppQObjectBlocks> {
    let mut generated = GeneratedCppQObjectBlocks::default();

    for &signal in signals {
        let data = generate_cpp_signal(signal, &qobject_idents.name, type_names, opt)?;
        let mut block = GeneratedCppQObjectBlocks {
            includes: data.includes,
            forward_declares_namespaced: data.forward_declares,
            fragments: data.fragments,
            methods: data.methods,
            ..Default::default()
        };
        generated.append(&mut block);
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::generator::cpp::property::tests::{require_header, require_pair};
    use crate::generator::naming::qobject::tests::create_qobjectname;
    use crate::tests::CfgEvaluatorTest;
    use indoc::indoc;
    use pretty_assertions::assert_str_eq;
    use syn::{parse_quote, ForeignItemFn};

    #[test]
    fn test_generate_cpp_signal_cfg() {
        let method: ForeignItemFn = parse_quote! {
            #[cfg(test_cfg_disabled)]
            #[cxx_name = "dataChanged"]
            fn data_changed(self: Pin<&mut MyObject>, trivial: i32, opaque: UniquePtr<QColor>);
        };
        let signal = ParsedSignal::mock(&method);
        let signals = vec![&signal];
        let qobject_idents = create_qobjectname();
        let type_names = TypeNames::mock();
        let opt = GeneratedOpt {
            cfg_evaluator: Box::new(CfgEvaluatorTest::default()),
        };
        let generated = generate_cpp_signals(&signals, &qobject_idents, &type_names, &opt).unwrap();

        assert_eq!(generated.methods.len(), 0);
    }

    #[test]
    fn test_generate_cpp_signals() {
        let method: ForeignItemFn = parse_quote! {
            #[cxx_name = "dataChanged"]
            fn data_changed(self: Pin<&mut MyObject>, trivial: i32, opaque: UniquePtr<QColor>);
        };
        let signal = ParsedSignal::mock(&method);
        let signals = vec![&signal];
        let qobject_idents = create_qobjectname();

        let mut type_names = TypeNames::mock();
        type_names.mock_insert("QColor", None, None, None);
        let generated = generate_cpp_signals(
            &signals,
            &qobject_idents,
            &type_names,
            &GeneratedOpt::default(),
        )
        .unwrap();

        assert_eq!(generated.methods.len(), 1);
        let header = require_header(&generated.methods[0]).unwrap();

        assert_str_eq!(
            header,
            "Q_SIGNAL void dataChanged(::std::int32_t trivial, ::std::unique_ptr<QColor> opaque);"
        );

        assert_eq!(generated.fragments.len(), 1);
        let (header, source) = require_pair(&generated.fragments[0]).unwrap();

        assert_str_eq!(
            header,
            indoc! {r#"
            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_dataChangedConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlerdataChanged closure, ::Qt::ConnectionType type);
            } // namespace rust::cxxqtgen1
            "#}
        );
        assert_str_eq!(
            source,
            indoc! {r#"
            // Define namespace otherwise we hit a GCC bug
            // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56480
            namespace rust::cxxqt1 {
            template <>
            SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>::~SignalHandler() noexcept
            {
                if (data[0] == nullptr && data[1] == nullptr)
                {
                    return;
                }

                drop_MyObject_signal_handler_dataChanged(::std::move(*this));
            }

            template <>
            template <>
            void SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>::operator()<MyObject&, ::std::int32_t, ::std::unique_ptr<QColor>>(MyObject& self, ::std::int32_t trivial, ::std::unique_ptr<QColor> opaque)
            {
                call_MyObject_signal_handler_dataChanged(*this, self, ::std::move(trivial), ::std::move(opaque));
            }

            static_assert(alignof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>) <= alignof(::std::size_t), "unexpected aligment");
            static_assert(sizeof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>) == sizeof(::std::size_t[2]), "unexpected size");
            } // namespace rust::cxxqt1

            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_dataChangedConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlerdataChanged closure, ::Qt::ConnectionType type)
            {
                return ::QObject::connect(
                    &self,
                    &MyObject::dataChanged,
                    &self,
                    [&, closure = ::std::move(closure)](::std::int32_t trivial, ::std::unique_ptr<QColor> opaque) mutable {
                        closure.template operator()<MyObject&, ::std::int32_t, ::std::unique_ptr<QColor>>(self, ::std::move(trivial), ::std::move(opaque));
                    },
                    type);
            }
            } // namespace rust::cxxqtgen1
            "#}
        );
    }

    #[test]
    fn test_generate_cpp_signals_mapped_cxx_name() {
        let method: ForeignItemFn = parse_quote! {
            #[cxx_name = "dataChanged"]
            fn data_changed(self: Pin<&mut MyObject>, mapped: A);
        };
        let signal = ParsedSignal::mock(&method);
        let signals = vec![&signal];
        let qobject_idents = create_qobjectname();

        let mut type_names = TypeNames::mock();
        type_names.mock_insert("A", None, Some("A1"), None);

        let generated = generate_cpp_signals(
            &signals,
            &qobject_idents,
            &type_names,
            &GeneratedOpt::default(),
        )
        .unwrap();

        assert_eq!(generated.methods.len(), 1);
        let header = require_header(&generated.methods[0]).unwrap();
        assert_str_eq!(header, "Q_SIGNAL void dataChanged(A1 mapped);");

        assert_eq!(generated.fragments.len(), 1);
        let (header, source) = require_pair(&generated.fragments[0]).unwrap();

        assert_str_eq!(
            header,
            indoc! {r#"
            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_dataChangedConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlerdataChanged closure, ::Qt::ConnectionType type);
            } // namespace rust::cxxqtgen1
            "#}
        );
        assert_str_eq!(
            source,
            indoc! {r#"
            // Define namespace otherwise we hit a GCC bug
            // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56480
            namespace rust::cxxqt1 {
            template <>
            SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>::~SignalHandler() noexcept
            {
                if (data[0] == nullptr && data[1] == nullptr)
                {
                    return;
                }

                drop_MyObject_signal_handler_dataChanged(::std::move(*this));
            }

            template <>
            template <>
            void SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>::operator()<MyObject&, A1>(MyObject& self, A1 mapped)
            {
                call_MyObject_signal_handler_dataChanged(*this, self, ::std::move(mapped));
            }

            static_assert(alignof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>) <= alignof(::std::size_t), "unexpected aligment");
            static_assert(sizeof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsdataChanged *>) == sizeof(::std::size_t[2]), "unexpected size");
            } // namespace rust::cxxqt1

            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_dataChangedConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlerdataChanged closure, ::Qt::ConnectionType type)
            {
                return ::QObject::connect(
                    &self,
                    &MyObject::dataChanged,
                    &self,
                    [&, closure = ::std::move(closure)](A1 mapped) mutable {
                        closure.template operator()<MyObject&, A1>(self, ::std::move(mapped));
                    },
                    type);
            }
            } // namespace rust::cxxqtgen1
            "#}
        );
    }

    #[test]
    fn test_generate_cpp_signals_existing_cxx_name() {
        let method: ForeignItemFn = parse_quote! {
                #[cxx_name = "baseName"]
                fn existing_signal(self: Pin<&mut MyObject>);
        };
        let signal = ParsedSignal {
            inherit: true,
            ..ParsedSignal::mock(&method)
        };

        let signals = vec![&signal];
        let qobject_idents = create_qobjectname();
        let generated = generate_cpp_signals(
            &signals,
            &qobject_idents,
            &TypeNames::mock(),
            &GeneratedOpt::default(),
        )
        .unwrap();

        assert_eq!(generated.methods.len(), 0);
        assert_eq!(generated.fragments.len(), 1);

        let (header, source) = require_pair(&generated.fragments[0]).unwrap();
        assert_str_eq!(
            header,
            indoc! {r#"
            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_baseNameConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlerbaseName closure, ::Qt::ConnectionType type);
            } // namespace rust::cxxqtgen1
            "#}
        );
        assert_str_eq!(
            source,
            indoc! {r#"
            // Define namespace otherwise we hit a GCC bug
            // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56480
            namespace rust::cxxqt1 {
            template <>
            SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsbaseName *>::~SignalHandler() noexcept
            {
                if (data[0] == nullptr && data[1] == nullptr)
                {
                    return;
                }

                drop_MyObject_signal_handler_baseName(::std::move(*this));
            }

            template <>
            template <>
            void SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsbaseName *>::operator()<MyObject&>(MyObject& self)
            {
                call_MyObject_signal_handler_baseName(*this, self);
            }

            static_assert(alignof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsbaseName *>) <= alignof(::std::size_t), "unexpected aligment");
            static_assert(sizeof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamsbaseName *>) == sizeof(::std::size_t[2]), "unexpected size");
            } // namespace rust::cxxqt1

            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_baseNameConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlerbaseName closure, ::Qt::ConnectionType type)
            {
                return ::QObject::connect(
                    &self,
                    &MyObject::baseName,
                    &self,
                    [&, closure = ::std::move(closure)]() mutable {
                        closure.template operator()<MyObject&>(self);
                    },
                    type);
            }
            } // namespace rust::cxxqtgen1
            "#}
        );
    }

    #[test]
    fn test_generate_cpp_signal_free() {
        let method: ForeignItemFn = parse_quote! {
            #[cxx_name = "signalRustName"]
            fn signal_rust_name(self: Pin<&mut MyObject>);
        };
        let signal = ParsedSignal {
            inherit: true,
            ..ParsedSignal::mock(&method)
        };

        let mut type_names = TypeNames::default();
        type_names.mock_insert("MyObject", None, None, None);
        let qobject_name = type_names.lookup(&signal.qobject_ident).unwrap();
        let generated =
            generate_cpp_signal(&signal, qobject_name, &type_names, &GeneratedOpt::default())
                .unwrap();

        assert_eq!(generated.methods.len(), 0);

        assert_eq!(generated.fragments.len(), 1);
        let (header, source) = require_pair(&generated.fragments[0]).unwrap();

        assert_str_eq!(
            header,
            indoc! {
            r#"
            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_signalRustNameConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlersignalRustName closure, ::Qt::ConnectionType type);
            } // namespace rust::cxxqtgen1
            "#}
        );
        assert_str_eq!(
            source,
            indoc! {r#"
            // Define namespace otherwise we hit a GCC bug
            // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56480
            namespace rust::cxxqt1 {
            template <>
            SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalRustName *>::~SignalHandler() noexcept
            {
                if (data[0] == nullptr && data[1] == nullptr)
                {
                    return;
                }

                drop_MyObject_signal_handler_signalRustName(::std::move(*this));
            }

            template <>
            template <>
            void SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalRustName *>::operator()<MyObject&>(MyObject& self)
            {
                call_MyObject_signal_handler_signalRustName(*this, self);
            }

            static_assert(alignof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalRustName *>) <= alignof(::std::size_t), "unexpected aligment");
            static_assert(sizeof(SignalHandler<::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalRustName *>) == sizeof(::std::size_t[2]), "unexpected size");
            } // namespace rust::cxxqt1

            namespace rust::cxxqtgen1 {
            ::QMetaObject::Connection
            MyObject_signalRustNameConnect(MyObject& self, ::rust::cxxqtgen1::MyObjectCxxQtSignalHandlersignalRustName closure, ::Qt::ConnectionType type)
            {
                return ::QObject::connect(
                    &self,
                    &MyObject::signalRustName,
                    &self,
                    [&, closure = ::std::move(closure)]() mutable {
                        closure.template operator()<MyObject&>(self);
                    },
                    type);
            }
            } // namespace rust::cxxqtgen1
            "#}
        );
    }

    #[test]
    fn test_generate_cpp_signal_free_mapped() {
        let method = parse_quote! {
            #[cxx_name = "signalCxxName"]
            fn signal_rust_name(self: Pin<&mut MyObject>);
        };
        let signal = ParsedSignal {
            inherit: true,
            ..ParsedSignal::mock(&method)
        };

        let mut type_names = TypeNames::default();
        type_names.mock_insert("MyObject", None, Some("ObjCpp"), Some("mynamespace"));
        let qobject_name = type_names.lookup(&signal.qobject_ident).unwrap();
        let generated =
            generate_cpp_signal(&signal, qobject_name, &type_names, &GeneratedOpt::default())
                .unwrap();

        assert_eq!(generated.methods.len(), 0);

        assert_eq!(generated.fragments.len(), 1);
        let (header, source) = require_pair(&generated.fragments[0]).unwrap();

        assert_str_eq!(
            header,
            indoc! {
            r#"
            namespace mynamespace::rust::cxxqtgen1 {
            ::QMetaObject::Connection
            ObjCpp_signalCxxNameConnect(mynamespace::ObjCpp& self, ::mynamespace::rust::cxxqtgen1::MyObjectCxxQtSignalHandlersignalCxxName closure, ::Qt::ConnectionType type);
            } // namespace mynamespace::rust::cxxqtgen1
            "#}
        );
        assert_str_eq!(
            source,
            indoc! {r#"
            // Define namespace otherwise we hit a GCC bug
            // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56480
            namespace rust::cxxqt1 {
            template <>
            SignalHandler<::mynamespace::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalCxxName *>::~SignalHandler() noexcept
            {
                if (data[0] == nullptr && data[1] == nullptr)
                {
                    return;
                }

                drop_MyObject_signal_handler_signalCxxName(::std::move(*this));
            }

            template <>
            template <>
            void SignalHandler<::mynamespace::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalCxxName *>::operator()<mynamespace::ObjCpp&>(mynamespace::ObjCpp& self)
            {
                call_MyObject_signal_handler_signalCxxName(*this, self);
            }

            static_assert(alignof(SignalHandler<::mynamespace::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalCxxName *>) <= alignof(::std::size_t), "unexpected aligment");
            static_assert(sizeof(SignalHandler<::mynamespace::rust::cxxqtgen1::MyObjectCxxQtSignalParamssignalCxxName *>) == sizeof(::std::size_t[2]), "unexpected size");
            } // namespace rust::cxxqt1

            namespace mynamespace::rust::cxxqtgen1 {
            ::QMetaObject::Connection
            ObjCpp_signalCxxNameConnect(mynamespace::ObjCpp& self, ::mynamespace::rust::cxxqtgen1::MyObjectCxxQtSignalHandlersignalCxxName closure, ::Qt::ConnectionType type)
            {
                return ::QObject::connect(
                    &self,
                    &mynamespace::ObjCpp::signalCxxName,
                    &self,
                    [&, closure = ::std::move(closure)]() mutable {
                        closure.template operator()<mynamespace::ObjCpp&>(self);
                    },
                    type);
            }
            } // namespace mynamespace::rust::cxxqtgen1
            "#}
        );
    }
}
