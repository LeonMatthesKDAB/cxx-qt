// SPDX-FileCopyrightText: 2021 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
// SPDX-FileContributor: Gerhard de Clercq <gerhard.declercq@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qguiapplication.h");
        type QGuiApplication;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        #[doc(hidden)]
        #[rust_name = "qguiapplication_exec"]
        fn qguiapplicationExec(app: Pin<&mut QGuiApplication>) -> i32;
        #[doc(hidden)]
        #[rust_name = "qguiapplication_new"]
        fn qguiapplicationNew(args: Vec<String>) -> UniquePtr<QGuiApplication>;
    }

    impl UniquePtr<QGuiApplication> {}
}

pub use ffi::QGuiApplication;

impl QGuiApplication {
    /// Enters the main event loop and waits until exit() is called,
    /// and then returns the value that was set to exit() (which is 0 if exit() is called via quit()).
    pub fn exec(self: std::pin::Pin<&mut QGuiApplication>) -> i32 {
        ffi::qguiapplication_exec(self)
    }

    pub fn new(args: Vec<String>) -> cxx::UniquePtr<Self> {
        ffi::qguiapplication_new(args)
    }

    pub fn new_from_args_os(args: std::env::ArgsOs) -> cxx::UniquePtr<Self> {
        Self::new(
            args.map(|string| {
                // Windows OsStrings are WTF-8 encoded, so they need to be
                // converted to UTF-8 Strings before being converted to Strings.
                // https://simonsapin.github.io/wtf-8/
                //
                // TODO: is this part still correct?
                #[cfg(windows)]
                return string.to_string_lossy();

                // TODO: is this part correct? as Qt can handle UTF8?
                string.into_string().unwrap()
            })
            .collect(),
        )
    }
}
