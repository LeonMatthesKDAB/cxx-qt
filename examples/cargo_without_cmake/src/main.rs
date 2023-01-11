// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Be Wilson <be.wilson@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
// SPDX-FileContributor: Gerhard de Clercq <gerhard.declercq@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// ANCHOR: book_cargo_imports
mod cxxqt_object;
// ANCHOR_END: book_cargo_imports

// ANCHOR: book_cargo_extern_c
extern "C" {
    fn run_cpp();
}
// ANCHOR_END: book_cargo_extern_c

// ANCHOR: book_cargo_rust_main
fn main() {
    let mut app = cxx_qt_lib::QGuiApplication::new_from_args_os(std::env::args_os());

    // Call the C++ initialization code to start the QML GUI.
    unsafe {
        run_cpp();
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
// ANCHOR_END: book_cargo_rust_main
